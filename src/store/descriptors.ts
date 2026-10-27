import { create } from "zustand";

import type { NodeCategory, NodeDescriptor } from "@/lib/types";
import { nodeCategories } from "@/lib/catalog";

interface DescriptorState {
  list: NodeDescriptor[];
  categories: NodeCategory[];
  setCategories: (categories: NodeCategory[]) => void;
  byId: Record<string, NodeDescriptor>;
  setDescriptors: (list: NodeDescriptor[]) => void;
}

/** Node descriptors loaded once at startup; drives palette + generic rendering. */
export const useDescriptorStore = create<DescriptorState>((set) => ({
  list: [],
  categories: nodeCategories,
  setCategories: (categories) => set({ categories }),
  byId: {},
  setDescriptors: (list) =>
    set({
      list,
      byId: Object.fromEntries(list.map((d) => [d.id, d])),
    }),
}));
