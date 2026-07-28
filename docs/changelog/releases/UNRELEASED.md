# 未发布

## 更新：固定预设部门与能力资产约束

- 功能（fixed-preset-departments）：新增固定的 explorer、reviewer 与 saddler 部门，将其固定为主助理的直属下级，并预置各自独立的模型、工具和 Skill 白名单；配置加载会自动补齐或修正这些内置部门与关联。
- 修复（saddler-pai-scope）：为 saddler 部门的文件与终端工具增加运行时范围校验，仅允许在当前项目 `.pai/` 目录内写入或更新能力资产。
