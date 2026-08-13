import { create } from 'zustand';

export interface AssistantMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  // Adjustments that were actually applied to the image for this message (if any).
  appliedAdjustments?: Record<string, number> | null;
  // Text metadata fields (EXIF keys) that were written for this message (if any).
  appliedMetadata?: Record<string, string> | null;
  // Human-readable summary of tag/rating/color-label changes applied (if any).
  appliedOrganization?: string | null;
  // How many images were attached to this (user) message.
  imageCount?: number;
  isError?: boolean;
}

interface AssistantState {
  messages: Array<AssistantMessage>;
  isLoading: boolean;
  addMessage: (message: AssistantMessage) => void;
  setLoading: (value: boolean) => void;
  clear: () => void;
}

let idCounter = 0;
export const nextMessageId = (): string => {
  idCounter += 1;
  return `assistant-${Date.now()}-${idCounter}`;
};

export const useAssistantStore = create<AssistantState>((set) => ({
  messages: [],
  isLoading: false,
  addMessage: (message) => set((state) => ({ messages: [...state.messages, message] })),
  setLoading: (value) => set({ isLoading: value }),
  clear: () => set({ messages: [], isLoading: false }),
}));
