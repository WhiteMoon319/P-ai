fn build_memory_generation_instruction(agent: &AgentProfile, user_alias: &str) -> String {
    let must_rules = format!(
        "A) 输出必须是合法 JSON，且仅包含 title/summary/openLoops/usefulMemoryIds/memoryActions 五个字段。\n\
         B) 不得输出 markdown、代码块、解释性前后缀。\n\
         C) 你是 {assistant_name}，用户称谓是 {user_name}。",
        assistant_name = agent.name,
        user_name = user_alias
    );
    let memory_rules = "1) openLoops 是仍需后续继续推进的事项列表；没有则输出 []；不要把 openLoops 里的内容再重复写成独立字段。\n\
         2) usefulMemoryIds 只能从“本次会话使用过的记忆”中选择；若上下文里看到的是短ID（如 12），就直接输出该短ID。\n\
         3) memoryActions 最多 7 条；每一项都必须带 action 与 memory；memory.memoryType 只能是 knowledge/skill/emotion/event。\n\
         4) memoryActions.action 只能是 create、update 或 merge。\n\
         5) 当 action=create 时，sourceMemoryIds 必须省略或输出 []。\n\
         6) 当 action=update 或 action=merge 时，sourceMemoryIds 必须填写，且只能引用“本次会话使用过的记忆”；若上下文里看到的是短ID，就直接输出短ID。\n\
         7) action=update 用于“旧记忆与当前事实冲突，需要替换”；action=merge 用于“多条旧记忆语义重复或需要合并”。\n\
         8) memory 表示该 action 处理完成后应保留的最终记忆表述；不要再拆出其他 target/source 外置结构。\n\
         9) 若某条 memory 属于用户画像，仍然放在 memoryActions 里，不再单独输出其他字段。\n\
         10) 用户画像记忆必须同时带有三个标签：profile、user_id:<真实用户ID字符串>、profile_attr:<属性类型>。\n\
         11) profile_attr 只能表达稳定画像属性，例如 alias、fact、skill、relationship、project、preference。\n\
         12) 画像记忆严禁使用 emotion；其 memoryType 只能是 knowledge、skill、event。\n\
         13) 画像记忆严禁放短期任务、临时情绪、一次性即时上下文。\n\
         14) 只有 user_id:* 允许出现真实 ID 字符串；除此之外，其他 tags 严禁出现数字、QQ号、手机号、群号、平台账号、数据库ID、消息ID、订单号等编号型内容。\n\
         15) memoryActions.memory 中的 judgment/reasoning/tags 必须使用当前用户本轮使用的语言（专有名词除外），禁止夹杂其他语言。\n\
         16) reasoning 定义：给出“支撑该 judgment 的论据/证据”；若没有可靠理由可以留空。\n\
         17) reasoning 必须写“支撑该 judgment 的论据/证据”，不得写流程话术。\n\
         18) reasoning 只允许描述对话中可追溯的依据，不得写“为了归档/为了生成记忆”。\n\
         19) reasoning 应尽量简洁具体；若没有可靠理由或证据不足，可留空字符串。\n\
         20) judgment 必须能被 reasoning 支撑；若无法支撑，宁可不生成该条记忆。\n\
         21) tags 中的每一项都必须是独立、紧凑、稳定、可检索的词元；不要写整句，不要写短语拼接，不要写“用户喜欢极简风格”这类带关系的长表达。\n\
         22) tags 只写检索锚点本身，例如 人名、项目名、偏好词、主题词、技能词、物品名；同一项里不要混入多个语义。\n\
         23) 不要记录高风险敏感信息（密码、密钥、身份证、银行卡等）。";
    format!(
        "你要做记忆整理。输出严格 JSON，不要 markdown，不要代码块。\n{}\n{}",
        prompt_xml_block("memory curation must", must_rules),
        prompt_xml_block("memory curation rules", memory_rules)
    )
}

fn memory_curation_example_output_block() -> &'static str {
    r###"{
  "title": "string",
  "summary": "string",
  "openLoops": ["string"],
  "usefulMemoryIds": ["12"],
  "memoryActions": [
    {
      "action": "create",
      "memory": {
        "memoryType": "knowledge|skill|emotion|event",
        "judgment": "string",
        "reasoning": "string",
        "tags": ["string"]
      }
    },
    {
      "action": "update",
      "sourceMemoryIds": ["12"],
      "memory": {
        "memoryType": "knowledge|skill|emotion|event",
        "judgment": "string",
        "reasoning": "string",
        "tags": ["string"]
      }
    },
    {
      "action": "merge",
      "sourceMemoryIds": ["12", "19"],
      "memory": {
        "memoryType": "knowledge|skill|emotion|event",
        "judgment": "string",
        "reasoning": "string",
        "tags": ["string"]
      }
    },
    {
      "action": "create",
      "memory": {
        "memoryType": "knowledge|skill|event",
        "judgment": "string",
        "reasoning": "string",
        "tags": ["profile", "user_id:o9cq80-MfLBeC-BBD-hStiFtlJSk@im.wechat", "profile_attr:fact", "深圳"]
      }
    }
  ]
}"###
}
