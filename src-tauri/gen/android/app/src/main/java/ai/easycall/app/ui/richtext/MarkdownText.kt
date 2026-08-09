package ai.easycall.app.ui.richtext

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ProvideTextStyle
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withLink
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.sp
import org.intellij.markdown.flavours.gfm.GFMFlavourDescriptor
import org.intellij.markdown.html.HtmlGenerator
import org.intellij.markdown.parser.MarkdownParser
import org.jsoup.Jsoup
import org.jsoup.nodes.Element
import org.jsoup.nodes.Node
import org.jsoup.nodes.TextNode

/**
 * 轻量 Markdown 渲染（复刻 RikkaHub 渲染路径）：
 * JetBrains markdown 解析 -> HTML -> Jsoup DOM -> Compose 逐块渲染。
 * RikkaHub 正是用 org.intellij.markdown + HtmlGenerator 把 markdown 转 HTML，再用
 * HTML 解析逐块渲染到 Compose 上。这里保留其核心方法，去掉其私有依赖(Latex/代码高亮/
 * 图标/表格增强)，用 Material3 默认样式实现常用语法。
 * 支持：标题/段落/粗斜体/删除线/行内代码/代码块/有序+无序列表/引用/表格/链接/分割线。
 */
@Composable
fun MarkdownText(
    content: String,
    modifier: Modifier = Modifier,
    style: TextStyle = androidx.compose.material3.LocalTextStyle.current,
) {
    val html = remember(content) { generateMarkdownHtml(content) }
    val document = remember(html) {
        runCatching { Jsoup.parse(html) }.getOrElse { Jsoup.parse("") }
    }
    val color = MaterialTheme.colorScheme

    ProvideTextStyle(style) {
        Column(modifier = modifier) {
            document.body().childNodes().forEach { node ->
                renderBodyNode(node = node, listLevel = 0, color = color)
            }
        }
    }
}

// ---- HTML generation ----

private val flavour by lazy {
    GFMFlavourDescriptor(makeHttpsAutoLinks = true, useSafeLinks = true)
}

private val parser by lazy { MarkdownParser(flavour) }

private fun generateMarkdownHtml(content: String): String {
    val tree = parser.buildMarkdownTreeFromString(content)
    return HtmlGenerator(content, tree, flavour).generateHtml()
}

// ---- Body node dispatch ----

@Composable
private fun renderBodyNode(node: Node, listLevel: Int, color: ColorScheme) {
    when (node) {
        is TextNode -> {
            val text = node.text().trim()
            if (text.isNotEmpty()) Text(text = text)
        }
        is Element -> renderBlockElement(element = node, listLevel = listLevel, color = color)
    }
}

@Composable
private fun renderBlockElement(element: Element, listLevel: Int, color: ColorScheme) {
    when (element.tagName().lowercase()) {
        "p" -> renderInline(element, color)
        "h1", "h2", "h3", "h4", "h5", "h6" -> renderHeading(element, color)
        "ul" -> renderList(element = element, ordered = false, level = listLevel, color = color)
        "ol" -> renderList(element = element, ordered = true, level = listLevel, color = color)
        "pre" -> renderCodeBlock(element, color)
        "blockquote" -> renderBlockquote(element, color)
        "table" -> renderTable(element, color)
        "hr" -> HorizontalDivider(
            modifier = Modifier.padding(vertical = 12.dp),
            color = color.onSurfaceVariant.copy(alpha = 0.4f),
            thickness = 0.5.dp,
        )
        else -> {
            Column(modifier = Modifier.fillMaxWidth()) {
                element.childNodes().forEach { renderBodyNode(it, listLevel, color) }
            }
        }
    }
}

@Composable
private fun renderInline(element: Element, color: ColorScheme) {
    val annotated = remember(element.outerHtml()) { buildInline(element.childNodes(), color) }
    if (annotated.isNotEmpty()) {
        Text(
            text = annotated,
            softWrap = true,
            overflow = TextOverflow.Visible,
            modifier = Modifier.fillMaxWidth().padding(vertical = 2.dp),
        )
    }
}

@Composable
private fun renderHeading(element: Element, color: ColorScheme) {
    val level = element.tagName().removePrefix("h").toIntOrNull() ?: 1
    val fontSize = when (level) {
        1 -> 24.sp; 2 -> 20.sp; 3 -> 17.sp; 4 -> 15.sp; else -> 14.sp
    }
    ProvideTextStyle(
        androidx.compose.material3.LocalTextStyle.current.copy(
            fontSize = fontSize,
            fontWeight = if (level <= 3) FontWeight.Bold else FontWeight.SemiBold,
        )
    ) {
        Box(modifier = Modifier.padding(top = 8.dp, bottom = 4.dp)) {
            val annotated = remember(element.outerHtml()) { buildInline(element.childNodes(), color) }
            Text(text = annotated)
        }
    }
}

@Composable
private fun renderList(element: Element, ordered: Boolean, level: Int, color: ColorScheme) {
    Column(modifier = Modifier.padding(start = (level * 8).dp, top = 2.dp, bottom = 2.dp)) {
        var idx = 1
        element.children().forEach { item ->
            if (item.tagName().lowercase() == "li") {
                val bullet = if (ordered) "${idx++}. " else "• "
                Row(verticalAlignment = Alignment.Top, modifier = Modifier.padding(vertical = 1.dp)) {
                    Text(bullet, color = color.primary, modifier = Modifier.alignByBaseline())
                    Column(modifier = Modifier.weight(1f)) {
                        val direct = item.childNodes().filter { node ->
                            !(node is Element && node.tagName().lowercase() in listOf("ul", "ol"))
                        }
                        if (direct.isNotEmpty()) {
                            val annotated = remember(item.outerHtml()) { buildInline(direct, color) }
                            Text(annotated, modifier = Modifier.fillMaxWidth())
                        }
                        item.children().forEach { child ->
                            val t = child.tagName().lowercase()
                            if (t == "ul" || t == "ol") {
                                renderList(child, ordered = t == "ol", level = level + 1, color = color)
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun renderCodeBlock(element: Element, color: ColorScheme) {
    val codeElement = element.selectFirst("code")
    val language = codeElement?.classNames()
        ?.find { it.startsWith("language-") }
        ?.removePrefix("language-")
        ?: "plaintext"
    val code = codeElement?.wholeText()?.trimEnd('\n')
        ?: element.wholeText().trimEnd('\n')

    Surface(
        color = color.surfaceVariant.copy(alpha = 0.5f),
        shape = RoundedCornerShape(6.dp),
        modifier = Modifier.fillMaxWidth().padding(vertical = 6.dp),
    ) {
        Column(modifier = Modifier.padding(10.dp)) {
            if (language != "plaintext") {
                Text(language, style = MaterialTheme.typography.labelSmall, color = color.primary)
            }
            Text(
                text = code,
                fontFamily = FontFamily.Monospace,
                fontSize = 13.sp,
                lineHeight = 18.sp,
                color = color.onSurface,
                modifier = Modifier.fillMaxWidth(),
            )
        }
    }
}

@Composable
private fun renderBlockquote(element: Element, color: ColorScheme) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp)
            .background(color.surfaceVariant.copy(alpha = 0.2f))
            .border(3.dp, color.onSurfaceVariant.copy(alpha = 0.4f))
            .padding(8.dp),
    ) {
        element.childNodes().forEach { renderBodyNode(it, 0, color) }
    }
}

@Composable
private fun renderTable(element: Element, color: ColorScheme) {
    val headers = element.select("thead tr th")
    val rows = element.select("tbody tr")
    if (headers.isEmpty() && rows.isEmpty()) {
        element.childNodes().forEach { renderBodyNode(it, 0, color) }
        return
    }
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 6.dp)
            .border(0.5.dp, color.onSurfaceVariant.copy(alpha = 0.4f)),
    ) {
        if (headers.isNotEmpty()) {
            Row(modifier = Modifier.fillMaxWidth().background(color.surfaceVariant.copy(alpha = 0.4f))) {
                headers.forEach { th ->
                    Text(
                        buildInline(th.childNodes(), color),
                        fontSize = 13.sp,
                        fontWeight = FontWeight.SemiBold,
                        modifier = Modifier.weight(1f).padding(6.dp),
                    )
                }
            }
        }
        rows.forEach { tr ->
            val cells = tr.select("td")
            if (cells.isNotEmpty()) {
                Row(modifier = Modifier.fillMaxWidth()) {
                    cells.forEach { td ->
                        Text(
                            buildInline(td.childNodes(), color),
                            fontSize = 13.sp,
                            modifier = Modifier.weight(1f).padding(horizontal = 6.dp, vertical = 4.dp),
                        )
                    }
                }
            }
        }
    }
}

// ---- Inline AnnotatedString ----

private fun buildInline(nodes: List<Node>, color: ColorScheme): AnnotatedString = buildAnnotatedString {
    fun rec(n: Node) {
        when (n) {
            is TextNode -> append(n.text())
            is Element -> when (n.tagName().lowercase()) {
                "b", "strong" -> withStyle(SpanStyle(fontWeight = FontWeight.Bold)) { n.childNodes().forEach { rec(it) } }
                "i", "em" -> withStyle(SpanStyle(fontStyle = FontStyle.Italic)) { n.childNodes().forEach { rec(it) } }
                "del", "s", "strike" -> withStyle(SpanStyle(textDecoration = TextDecoration.LineThrough)) { n.childNodes().forEach { rec(it) } }
                "u" -> withStyle(SpanStyle(textDecoration = TextDecoration.Underline)) { n.childNodes().forEach { rec(it) } }
                "code" -> withStyle(SpanStyle(fontFamily = FontFamily.Monospace, fontSize = 0.9.em, color = color.primary)) {
                    append(' '); append(n.text()); append(' ')
                }
                "a" -> {
                    val href = n.attr("href")
                    if (href.isNotEmpty()) {
                        withLink(LinkAnnotation.Url(href)) {
                            withStyle(SpanStyle(color = color.primary, textDecoration = TextDecoration.Underline)) {
                                n.childNodes().forEach { rec(it) }
                            }
                        }
                    } else {
                        n.childNodes().forEach { rec(it) }
                    }
                }
                "br" -> append("\n")
                else -> n.childNodes().forEach { rec(it) }
            }
        }
    }
    nodes.forEach { rec(it) }
}