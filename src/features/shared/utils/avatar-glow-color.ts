/** 光斑透明度：与头像叠加后是「发光」而不是「染色」 */
const GLOW_ALPHA = 0.19;
/** 采样尺寸：缩到这么小既够代表主色，也省一次大图解码开销 */
const SAMPLE_SIZE = 24;

/** 同一头像只取一次色，切换人格来回点时不重复解码 */
const glowColorCache = new Map<string, string | null>();

/**
 * 从头像图片提取主色，返回可直接用作光斑背景的 rgba 字符串。
 * 取不到时（跨域污染 canvas、解码失败、全透明图）返回 null，由调用方降级到主题色。
 */
export function extractAvatarGlowColor(url: string): Promise<string | null> {
  const normalized = String(url || "").trim();
  if (!normalized) return Promise.resolve(null);
  const cached = glowColorCache.get(normalized);
  if (cached !== undefined) return Promise.resolve(cached);
  return new Promise((resolve) => {
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.decoding = "async";
    img.onload = () => {
      const rgba = readDominantColor(img);
      glowColorCache.set(normalized, rgba);
      resolve(rgba);
    };
    img.onerror = () => {
      glowColorCache.set(normalized, null);
      resolve(null);
    };
    img.src = normalized;
  });
}

/** 中心区域优先采样，滤掉透明、近白、近黑与近灰，中心不足时回退全图平均 */
function readDominantColor(img: HTMLImageElement): string | null {
  try {
    const canvas = document.createElement("canvas");
    canvas.width = SAMPLE_SIZE;
    canvas.height = SAMPLE_SIZE;
    const ctx = canvas.getContext("2d", { willReadFrequently: true });
    if (!ctx) return null;
    ctx.drawImage(img, 0, 0, SAMPLE_SIZE, SAMPLE_SIZE);
    const data = ctx.getImageData(0, 0, SAMPLE_SIZE, SAMPLE_SIZE).data;

    let center = sampleRegion(data, 4, SAMPLE_SIZE - 4, 4, SAMPLE_SIZE - 4);
    if (center.count < 8) {
      center = sampleRegion(data, 0, SAMPLE_SIZE, 0, SAMPLE_SIZE);
    }
    if (!center.count) return null;

    const r = Math.round(center.r / center.count);
    const g = Math.round(center.g / center.count);
    const b = Math.round(center.b / center.count);
    return `rgba(${r}, ${g}, ${b}, ${GLOW_ALPHA})`;
  } catch {
    // 跨域图片会污染 canvas，getImageData 抛异常，此时交给调用方降级
    return null;
  }
}

function sampleRegion(
  data: Uint8ClampedArray,
  xStart: number,
  xEnd: number,
  yStart: number,
  yEnd: number,
) {
  let r = 0;
  let g = 0;
  let b = 0;
  let count = 0;
  for (let y = yStart; y < yEnd; y++) {
    for (let x = xStart; x < xEnd; x++) {
      const i = (y * SAMPLE_SIZE + x) * 4;
      if (data[i + 3] < 128) continue;
      const cr = data[i];
      const cg = data[i + 1];
      const cb = data[i + 2];
      if (cr > 250 && cg > 250 && cb > 250) continue;
      if (cr < 12 && cg < 12 && cb < 12) continue;
      // 近灰的像素会把主色洗淡，直接跳过
      if (Math.max(cr, cg, cb) - Math.min(cr, cg, cb) < 12) continue;
      r += cr;
      g += cg;
      b += cb;
      count++;
    }
  }
  return { r, g, b, count };
}
