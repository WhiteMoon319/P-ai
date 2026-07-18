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
};

export type ConfigTemplateRow = {
  items: ConfigTemplateField[];
};

export type ConfigTemplateGroup = {
  title: string;
  rows: ConfigTemplateRow[];
};
