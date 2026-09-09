import { describe, expect, it } from "vitest";
import type { DepartmentPermissionCatalogItem } from "../../../types/app";
import {
  buildBuiltinToolGroups,
  buildMcpToolGroups,
  normalizeDepartmentPermissionCatalog,
} from "./department-tool-tree";

function item(name: string, description = "", group = ""): DepartmentPermissionCatalogItem {
  return { name, description, group };
}

describe("buildBuiltinToolGroups", () => {
  it("按功能域分组并保持定义顺序", () => {
    const groups = buildBuiltinToolGroups(
      [item("read"), item("write"), item("exec"), item("fetch")],
      () => false,
      (key) => `label:${key}`,
    );
    expect(groups.map((group) => [group.label, group.leaves.map((leaf) => leaf.name)])).toEqual([
      ["label:files", ["read", "write"]],
      ["label:execConfig", ["exec"]],
      ["label:web", ["fetch"]],
    ]);
  });

  it("未知工具落入其他组", () => {
    const groups = buildBuiltinToolGroups([item("mystery_tool"), item("read")], () => false, (key) => key);
    const other = groups.find((group) => group.key === "other");
    expect(other?.leaves.map((leaf) => leaf.name)).toEqual(["mystery_tool"]);
  });

  it("空目录返回空分组", () => {
    expect(buildBuiltinToolGroups([], () => false, () => "x")).toEqual([]);
  });

  it("组状态按叶子启用情况聚合", () => {
    const groups = buildBuiltinToolGroups([item("read"), item("write")], (name) => name === "write", () => "x");
    const files = groups[0];
    expect(files.state).toBe("partial");
    expect(files.leaves.find((leaf) => leaf.name === "write")?.enabled).toBe(true);
    expect(files.leaves.find((leaf) => leaf.name === "read")?.enabled).toBe(false);
  });

  it("全部启用与全部禁用时的组状态", () => {
    const allOn = buildBuiltinToolGroups([item("read"), item("write")], () => true, () => "x");
    expect(allOn[0].state).toBe("all");
    const allOff = buildBuiltinToolGroups([item("read"), item("write")], () => false, () => "x");
    expect(allOff[0].state).toBe("none");
  });
});

describe("buildMcpToolGroups", () => {
  it("按 server 分组且叶子保留注册层全名", () => {
    const groups = buildMcpToolGroups(
      [item("read", "", "fs"), item("write", "", "fs"), item("status", "", "git")],
      () => false,
      "其他",
    );
    expect(groups.map((group) => group.label)).toEqual(["fs", "git"]);
    const fs = groups[0];
    expect(fs.leaves.map((leaf) => leaf.displayName)).toEqual(["read", "write"]);
    expect(fs.leaves.map((leaf) => leaf.name)).toEqual(["read", "write"]);
    expect(fs.state).toBe("none");
  });

  it("无分组工具落入其他组并使用传入的展示名", () => {
    const groups = buildMcpToolGroups([item("bare")], () => false, "其他");
    expect(groups).toHaveLength(1);
    expect(groups[0].key).toBe("other");
    expect(groups[0].label).toBe("其他");
    expect(groups[0].leaves[0]?.displayName).toBe("bare");
  });

  it("组内部分启用时状态为 partial", () => {
    const groups = buildMcpToolGroups(
      [item("a", "", "s"), item("b", "", "s")],
      (name) => name === "a",
      "其他",
    );
    expect(groups[0].state).toBe("partial");
  });

  it("server 名为 other 与无分组工具并存时分组键不冲突", () => {
    const groups = buildMcpToolGroups(
      [item("read", "", "other"), item("bare")],
      () => false,
      "其他",
    );
    expect(groups).toHaveLength(2);
    const keys = groups.map((group) => group.key);
    expect(new Set(keys).size).toBe(2);
    expect(keys).toContain("server:other");
    expect(keys).toContain("other");
    const serverGroup = groups.find((group) => group.key === "server:other");
    expect(serverGroup?.label).toBe("other");
    expect(serverGroup?.leaves.map((leaf) => leaf.displayName)).toEqual(["read"]);
    const fallbackGroup = groups.find((group) => group.key === "other");
    expect(fallbackGroup?.label).toBe("其他");
    expect(fallbackGroup?.leaves.map((leaf) => leaf.displayName)).toEqual(["bare"]);
  });
});

describe("normalizeDepartmentPermissionCatalog", () => {
  it("保留 MCP 工具的服务器分组字段并据此分组", () => {
    const catalog = normalizeDepartmentPermissionCatalog({
      builtinTools: [{ name: "read", description: "读取", group: "" }],
      skills: [{ name: "pai-guide", description: "指南", group: "" }],
      mcpTools: [
        { name: "akasha_books", description: "检索文档", group: "akasha" },
        { name: "playwright_browser_click", description: "点击", group: "playwright" },
      ],
    });
    expect(catalog.mcpTools.map((entry) => entry.group)).toEqual(["akasha", "playwright"]);
    const groups = buildMcpToolGroups(catalog.mcpTools, () => false, "其他");
    expect(groups.map((group) => group.label)).toEqual(["akasha", "playwright"]);
  });

  it("分组字段缺失、为空或含首尾空格时归入其他或去空格", () => {
    const catalog = normalizeDepartmentPermissionCatalog({
      mcpTools: [
        { name: "bare" },
        { name: "  ", description: "无名字", group: "akasha" },
        null,
        { name: "tavily_search", description: "搜索", group: "  tavily  " },
      ],
    });
    expect(catalog.mcpTools.map((entry) => entry.name)).toEqual(["bare", "tavily_search"]);
    expect(catalog.mcpTools[1]?.group).toBe("tavily");
    const groups = buildMcpToolGroups(catalog.mcpTools, () => false, "其他");
    expect(groups.map((group) => group.label)).toEqual(["tavily", "其他"]);
  });

  it("payload 缺字段或类型异常时返回空目录", () => {
    expect(normalizeDepartmentPermissionCatalog(null)).toEqual({
      builtinTools: [],
      skills: [],
      mcpTools: [],
    });
    expect(normalizeDepartmentPermissionCatalog({ builtinTools: "oops" }).builtinTools).toEqual([]);
  });
});
