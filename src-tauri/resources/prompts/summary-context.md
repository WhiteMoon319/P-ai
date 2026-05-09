<system_remind>
前面的所有消息都是历史对话内容，不是当前用户正在发起的新问题。
请不要回应、解释、评价或纠错历史消息本身，也不要把本条维护指令理解成用户在让你检查提示词。
你当前唯一任务是执行中场总结反思，并严格按照后续要求输出 JSON。
</system_remind>

<summary_requirement>
你正在执行一次中场总结反思。

这属于系统的上下文整理流程，但你在语义上要把它理解为：在对话中途停下来，同时完成上下文交接、未完事项整理、已用记忆筛选、以及新记忆生成判断。

你的工具都已经被禁用，你只能生成 JSON 完成任务。

title 表示“当前会话标题”，应概括本轮主题，尽量控制在 10 个汉字以内。

summary 请直接按下面顺序自然书写，不要额外发明字段名：
第一，先写当前进展，以及已经做出的关键决策。
第二，再写重要上下文、约束条件、用户偏好。
第三，补充后续工作需要回看的文件片段、网页片段、日志片段或引用材料。
- 只有在后续续跑明显还会用到时，才补充这部分。
- 优先保留最近刚读到、后续还要回看的片段。
- 每条尽量包含可回看的定位信息（如文件路径、行号、页面名、日志来源）。
- 每条都尽量附上短原文摘录，避免只写意译；若定位信息已足够，则摘录保持最短必要长度。
- 推荐格式：- `定位信息` 为什么重要
  原文：摘录

openLoops 单独填写剩余待办、未闭环事项与清晰下一步；没有则输出 []。
请保持内容简洁、结构化，并专注于帮助下一个语言模型无缝继续当前工作。
</summary_requirement>

<memory_curation_context>
你是 {{assistant_name}}，用户称谓是 {{user_name}}。

你还需要在同一次中场总结反思里判断记忆：
- usefulMemoryIds 表示本次上下文仍然有用的旧记忆。
- memoryActions 表示需要新增、修正或合并的记忆。

<memory generation rules>
{{memory_generation_rules}}
</memory generation rules>
</memory_curation_context>

<json_contract>
输出必须是合法 JSON，不要 markdown，不要代码块，不要解释性前后缀。

顶层仅允许以下五个字段：
- `title`
- `summary`
- `openLoops`
- `usefulMemoryIds`
- `memoryActions`

字段规则：
- `summary` 表示本次上下文检查点压缩的交接摘要，必须方便下一个模型继续当前任务直接使用；剩余待办、下一步与未闭环事项请写入 `openLoops`，不要再在 `summary` 里单独造字段。
- `openLoops` 是仍需后续继续推进的事项列表；没有则输出 `[]`。
- `usefulMemoryIds` 只能从“本次会话使用过的记忆”中选择；看到短 ID 就直接输出短 ID。
- `memoryActions` 最多 7 条。
- `memoryActions[*].action` 只能是 `create`、`update`、`merge`。
- 每个 `memoryActions` 项都必须包含 `memory`。
- `memory.memoryType` 只能是 `knowledge`、`skill`、`emotion`、`event`。
- `create` 省略 `sourceMemoryIds` 或输出 `[]`。
- `update` / `merge` 必须填写 `sourceMemoryIds`，且只能引用本次会话使用过的记忆。
- `memory` 表示处理后最终保留的记忆。
- `memory` 必须包含 `memoryType`、`judgment`、`reasoning`、`tags`。

EXAMPLE JSON OUTPUT:
{{json_example}}
</json_contract>
