export type ShareDocumentEntry = {
  id: string;
  align: "left" | "right";
  tone: "user" | "assistant";
  displayName: string;
  avatarUrl: string;
  createdAtText: string;
  text: string;
  thinkingSummary?: string;
  images?: Array<{ src: string; alt: string }>;
  attachmentNames?: string[];
  audioCount?: number;
};
