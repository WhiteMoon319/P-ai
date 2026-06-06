import { readFileSync, readdirSync, statSync } from 'fs';
import { join, relative } from 'path';

const srcDir = 'src';
const extensions = ['.vue', '.ts'];

function scanFile(filePath) {
  const content = readFileSync(filePath, 'utf8');
  const lines = content.split('\n');
  const results = [];
  
  lines.forEach((line, index) => {
    // Skip comments
    if (line.trim().startsWith('//') || line.trim().startsWith('*') || line.trim().startsWith('/*')) return;
    if (line.includes('<!--') && !line.includes('-->')) return;
    
    // Skip existing i18n calls
    if (line.includes('t(') || line.includes('$t(')) return;
    
    // Find Chinese characters
    const matches = line.match(/[\u4e00-\u9fff]+/g);
    if (matches) {
      results.push({
        line: index + 1,
        content: line.trim(),
        chinese: matches
      });
    }
  });
  
  return results;
}

function scanDir(dir) {
  const results = {};
  
  function walk(currentDir) {
    const entries = readdirSync(currentDir);
    
    for (const entry of entries) {
      const fullPath = join(currentDir, entry);
      const stat = statSync(fullPath);
      
      if (stat.isDirectory()) {
        if (!entry.startsWith('.') && entry !== 'node_modules' && entry !== 'scripts') {
          walk(fullPath);
        }
      } else if (extensions.some(ext => entry.endsWith(ext))) {
        const relPath = relative(srcDir, fullPath);
        const fileResults = scanFile(fullPath);
        if (fileResults.length > 0) {
          results[relPath] = fileResults;
        }
      }
    }
  }
  
  walk(dir);
  return results;
}

const results = scanDir(srcDir);

// Output summary
let totalFiles = 0;
let totalOccurrences = 0;

for (const [file, occurrences] of Object.entries(results)) {
  totalFiles++;
  totalOccurrences += occurrences.length;
}

console.log(`\n扫描完成：${totalFiles} 个文件，${totalOccurrences} 处硬编码中文\n`);

// Output by file
for (const [file, occurrences] of Object.entries(results).sort((a, b) => b[1].length - a[1].length)) {
  console.log(`\n${file} (${occurrences.length} 处):`);
  for (const occ of occurrences.slice(0, 5)) {
    console.log(`  L${occ.line}: ${occ.chinese.join(', ')}`);
  }
  if (occurrences.length > 5) {
    console.log(`  ... 还有 ${occurrences.length - 5} 处`);
  }
}
