import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type {
  FundSearchResult,
  Portfolio,
  PortfolioPosition,
  PortfolioVersion,
  PositionDiff,
} from "../types";

interface PortfolioState {
  portfolios: Portfolio[];
  /** portfolio_id -> 最新一期仓位（后端已按权重降序） */
  positions: Record<string, PortfolioPosition[]>;
  /** portfolio_id -> 版本列表（按报告期降序） */
  versions: Record<string, PortfolioVersion[]>;
  /** portfolio_id -> 最近一次请求的调仓对比结果 */
  diffs: Record<string, PositionDiff>;
  searchResults: FundSearchResult[];
  loading: boolean;
  searching: boolean;
  /** 正在刷新的组合 id（同一时刻只允许刷新一个） */
  refreshingId: string | null;
  error: string | null;
  searchError: string | null;
  fetchPortfolios: () => Promise<void>;
  searchFunds: (keyword: string) => Promise<void>;
  clearSearch: () => void;
  createFundPortfolio: (fund: FundSearchResult) => Promise<void>;
  deletePortfolio: (id: string) => Promise<void>;
  refreshPortfolio: (id: string) => Promise<void>;
  fetchPositions: (id: string) => Promise<void>;
  fetchVersions: (id: string) => Promise<PortfolioVersion[]>;
  fetchDiff: (id: string, fromDate?: string, toDate?: string) => Promise<PositionDiff>;
}

export const usePortfolioStore = create<PortfolioState>((set, get) => ({
  portfolios: [],
  positions: {},
  versions: {},
  diffs: {},
  searchResults: [],
  loading: false,
  searching: false,
  refreshingId: null,
  error: null,
  searchError: null,

  fetchPortfolios: async () => {
    set({ loading: true, error: null });
    try {
      const portfolios = await invoke<Portfolio[]>("list_portfolios");
      set({ portfolios, loading: false });
    } catch (err) {
      set({ error: String(err), loading: false });
    }
  },

  searchFunds: async (keyword) => {
    set({ searching: true, searchError: null });
    try {
      const searchResults = await invoke<FundSearchResult[]>("search_funds", { keyword });
      set({ searchResults, searching: false });
    } catch (err) {
      set({ searchError: String(err), searchResults: [], searching: false });
    }
  },

  clearSearch: () => set({ searchResults: [], searchError: null, searching: false }),

  createFundPortfolio: async (fund) => {
    try {
      await invoke<Portfolio>("create_fund_portfolio", {
        fundCode: fund.fund_code,
        fundName: fund.fund_name,
        fundType: fund.fund_type,
      });
    } finally {
      // 「组合已创建但首刷失败」也会走 Err 分支，因此无论成败都重取列表
      await get().fetchPortfolios();
    }
  },

  deletePortfolio: async (id) => {
    await invoke("delete_portfolio", { portfolioId: id });
    set((state) => {
      const positions = { ...state.positions };
      delete positions[id];
      return { portfolios: state.portfolios.filter((p) => p.id !== id), positions };
    });
  },

  refreshPortfolio: async (id) => {
    set({ refreshingId: id });
    try {
      const latest = await invoke<PortfolioPosition[]>("refresh_fund_portfolio", {
        portfolioId: id,
      });
      set((state) => ({
        positions: { ...state.positions, [id]: latest },
        refreshingId: null,
      }));
      await get().fetchPortfolios();
    } catch (err) {
      set({ refreshingId: null });
      throw err;
    }
  },

  fetchPositions: async (id) => {
    const latest = await invoke<PortfolioPosition[]>("get_portfolio_positions", {
      portfolioId: id,
    });
    set((state) => ({ positions: { ...state.positions, [id]: latest } }));
  },

  fetchVersions: async (id) => {
    const versions = await invoke<PortfolioVersion[]>("get_portfolio_versions", {
      portfolioId: id,
    });
    set((state) => ({ versions: { ...state.versions, [id]: versions } }));
    return versions;
  },

  fetchDiff: async (id, fromDate, toDate) => {
    const diff = await invoke<PositionDiff>("get_portfolio_diff", {
      portfolioId: id,
      fromDate,
      toDate,
    });
    set((state) => ({ diffs: { ...state.diffs, [id]: diff } }));
    return diff;
  },
}));
