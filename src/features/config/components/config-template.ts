export type ConfigTemplateControl = "toggle" | "select" | "text" | "number" | "textarea";

export type ConfigTemplateOption = {
  label: string;
  value: string;
};

export type ConfigTemplateField = {
  key: string;
  label: string;
  description?: string;
  type: ConfigTemplateControl;
  options?: ConfigTemplateOption[];
  placeholder?: string;
  min?: number;
  max?: number;
  step?: number;
  disabled?: boolean;
  stacked?: boolean;
};

export type ConfigTemplateRow = {
  key?: string;
  items: ConfigTemplateField[];
};

export type ConfigTemplateGroup = {
  key?: string;
  title: string;
  rows: ConfigTemplateRow[];
};
