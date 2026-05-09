<summary_requirement>
你正在执行一次归档反思。

这属于系统的归档流程，但你在语义上只需要回看本轮完整上下文，判断哪些旧记忆仍然有用，以及是否需要新增、修正或合并记忆。

你的工具都已经被禁用，你只能生成 JSON 完成任务。

不要生成归档摘要。
不要生成会话标题。
不要复述对话过程。
不要输出后续待办。
</summary_requirement>

<memory_curation_context>
你是 {{assistant_name}}，用户称谓是 {{user_name}}。

你只需要在这次归档反思里判断记忆：
- usefulMemoryIds 表示本次上下文仍然有用的旧记忆。
- memoryActions 表示需要新增、修正或合并的记忆。

<memory generation rules>
{{memory_generation_rules}}
</memory generation rules>
</memory_curation_context>

<json_contract>
输出必须是合法 JSON，不要 markdown，不要代码块，不要解释性前后缀。

系统只读取以下两个字段：
- `usefulMemoryIds`
- `memoryActions`

字段规则：
- 你不需要生成 `title`、`summary`、`openLoops`；如果因旧格式习惯额外输出这些字段，系统会忽略。
- `usefulMemoryIds` 只能从“本次会话使用过的记忆”中选择；看到短 ID 就直接输出短 ID。
- `memoryActions` 最多 7 条。
- `memoryActions[*].action` 只能是 `create`、`update`、`merge`。
- 每个 `memoryActions` 项都必须包含 `memory`。
- `memory.memoryType` 只能是 `knowledge`、`skill`、`emotion`、`event`。
- `create` 省略 `sourceMemoryIds` 或输出 `[]`。
- `update` / `merge` 必须填写 `sourceMemoryIds`，且只能引用本次会话使用过的记忆。
- `memory` 表示处理后最终保留的记忆。
- `memory` 必须包含 `memoryType`、`judgment`、`reasoning`、`tags`。

下面是唯一合法的 JSON 形状示例：
{{json_example}}
</json_contract>
