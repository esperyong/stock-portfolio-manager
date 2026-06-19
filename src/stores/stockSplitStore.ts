import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { StockSplit } from "../types";

interface StockSplitState {
  splits: StockSplit[];
  loading: boolean;
  error: string | null;

  fetchSplits: () => Promise<void>;
  addSplit: (stockCode: string, splitDate: string, ratioFrom: number, ratioTo: number) => Promise<StockSplit>;
  deleteSplit: (id: number) => Promise<void>;
}

export const useStockSplitStore = create<StockSplitState>((set) => ({
  splits: [],
  loading: false,
  error: null,

  fetchSplits: async () => {
    set({ loading: true, error: null });
    try {
      const splits = await invoke<StockSplit[]>("get_stock_splits");
      set({ splits, loading: false });
    } catch (err) {
      set({ error: String(err), loading: false });
    }
  },

  addSplit: async (stockCode: string, splitDate: string, ratioFrom: number, ratioTo: number) => {
    const split = await invoke<StockSplit>("add_stock_split", {
      stockCode,
      splitDate,
      ratioFrom,
      ratioTo,
    });
    set((state) => ({ splits: [split, ...state.splits] }));
    return split;
  },

  deleteSplit: async (id: number) => {
    await invoke("delete_stock_split", { id });
    set((state) => ({ splits: state.splits.filter((s) => s.id !== id) }));
  },
}));
