import { create } from 'zustand';
import { persist } from 'zustand/middleware';

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

export interface Conversation {
  id: string;
  title: string;
  messages: Array<AssistantMessage>;
  createdAt: number;
  updatedAt: number;
}

interface AssistantState {
  conversations: Array<Conversation>;
  activeId: string | null;
  isLoading: boolean;
  // Appends to the active conversation, creating one on demand.
  addMessage: (message: AssistantMessage) => void;
  setLoading: (value: boolean) => void;
  newConversation: () => string;
  selectConversation: (id: string) => void;
  renameConversation: (id: string, title: string) => void;
  deleteConversation: (id: string) => void;
  // Empties the active conversation (keeps the row, resets its title).
  clearActive: () => void;
  // Replace the active conversation's messages wholesale (used by /compact).
  replaceActiveMessages: (messages: Array<AssistantMessage>) => void;
}

let idCounter = 0;
export const nextMessageId = (): string => {
  idCounter += 1;
  return `assistant-${Date.now()}-${idCounter}`;
};

let convCounter = 0;
const nextConversationId = (): string => {
  convCounter += 1;
  return `conv-${Date.now()}-${convCounter}`;
};

const DEFAULT_TITLE = 'New chat';

function makeConversation(): Conversation {
  const now = Date.now();
  return { id: nextConversationId(), title: DEFAULT_TITLE, messages: [], createdAt: now, updatedAt: now };
}

// Title an untitled conversation from its first user message, ChatGPT-style.
function deriveTitle(text: string): string {
  const clean = text.replace(/\s+/g, ' ').trim();
  if (!clean) return DEFAULT_TITLE;
  return clean.length > 40 ? `${clean.slice(0, 40)}…` : clean;
}

export const useAssistantStore = create<AssistantState>()(
  persist(
    (set, get) => ({
      conversations: [],
      activeId: null,
      isLoading: false,

      addMessage: (message) =>
        set((state) => {
          let conversations = state.conversations;
          let activeId = state.activeId;
          if (!activeId || !conversations.find((c) => c.id === activeId)) {
            const conv = makeConversation();
            conversations = [conv, ...conversations];
            activeId = conv.id;
          }
          conversations = conversations.map((c) => {
            if (c.id !== activeId) return c;
            const isFirstUser = message.role === 'user' && !c.messages.some((m) => m.role === 'user');
            return {
              ...c,
              title: c.title === DEFAULT_TITLE && isFirstUser ? deriveTitle(message.content) : c.title,
              messages: [...c.messages, message],
              updatedAt: Date.now(),
            };
          });
          return { conversations, activeId };
        }),

      setLoading: (value) => set({ isLoading: value }),

      newConversation: () => {
        // Reuse an existing empty conversation instead of stacking blanks.
        const existingEmpty = get().conversations.find((c) => c.messages.length === 0);
        if (existingEmpty) {
          set({ activeId: existingEmpty.id });
          return existingEmpty.id;
        }
        const conv = makeConversation();
        set((state) => ({ conversations: [conv, ...state.conversations], activeId: conv.id }));
        return conv.id;
      },

      selectConversation: (id) => set({ activeId: id }),

      renameConversation: (id, title) =>
        set((state) => ({
          conversations: state.conversations.map((c) =>
            c.id === id ? { ...c, title: title.trim() || DEFAULT_TITLE } : c,
          ),
        })),

      deleteConversation: (id) =>
        set((state) => {
          const conversations = state.conversations.filter((c) => c.id !== id);
          const activeId = state.activeId === id ? (conversations[0]?.id ?? null) : state.activeId;
          return { conversations, activeId };
        }),

      clearActive: () =>
        set((state) => ({
          conversations: state.conversations.map((c) =>
            c.id === state.activeId ? { ...c, messages: [], title: DEFAULT_TITLE, updatedAt: Date.now() } : c,
          ),
        })),

      replaceActiveMessages: (messages) =>
        set((state) => ({
          conversations: state.conversations.map((c) =>
            c.id === state.activeId ? { ...c, messages, updatedAt: Date.now() } : c,
          ),
        })),
    }),
    {
      name: 'rapidraw-assistant',
      // Persist the conversations, not the transient loading flag.
      partialize: (state) => ({ conversations: state.conversations, activeId: state.activeId }),
    },
  ),
);
