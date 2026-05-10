# Markdown 样式覆盖说明

PAI 的 Markdown 渲染样式按“语义样式全局化，场景排版局部化”维护。

## 共享语义样式

通用 Markdown 语义节点统一维护在：

- `src/features/chat/markdown/markdown-content.css`

新增 Markdown 渲染器时，应给渲染根节点挂上 `ecall-markdown-content`，即可复用默认主题样式，包括：

- 文字颜色继承
- 链接
- strong
- blockquote
- inline code
- table 基础边框与单元格
- hr
- Mermaid / code block 的基础可见性修正

不要在每个页面重复写这些语义节点的主题覆盖。

## 场景局部样式

各窗口只保留和具体场景有关的排版差异，例如：

- 字号
- 行高
- 段落间距
- 标题尺寸
- 列表缩进
- 表格 hover / stripe 等局部增强
- 文件阅读器、归档页等特殊阅读密度

这类样式应留在对应组件内，不要塞回全局共享层。

## 代码块渲染器

代码块组件按场景拆分：

- 聊天窗口：`src/features/chat/components/ChatShikiCodeBlockNode.vue`
- 文件阅读器：`src/apps/file-reader/FileReaderCodeBlockNode.vue`

不要把聊天窗口的 `chat-markstream` 代码块渲染器直接复用到文档阅读场景。聊天代码块依赖聊天气泡外壳，文件阅读器应使用自己的 `file-reader-markstream` 注册与代码块组件。

## 覆盖原则

1. 优先改共享语义样式，避免重复覆盖。
2. 只在场景确有差异时写局部规则。
3. 子组件内部 DOM 需要穿透时才使用 `:deep()`。
4. 避免无依据使用 `!important`；若必须覆盖第三方库 containment 或可见性规则，应先确认来源与影响范围。
5. 透明度只保留一层来源，避免 `color-mix(... transparent)` 与 `opacity` 叠加。
