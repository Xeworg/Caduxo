/**
 * TypeScript API wrapper for category search (caduxo-category-management).
 * Thin wrappers around the Rust command layer; no business logic here.
 */

import { invoke } from "@tauri-apps/api/core";
import type { CategoryResponse } from "./products.js";

/** Sentinel value for the "Uncategorized" pseudo-category filter option.
 * Products with zero active category memberships are included when this sentinel
 * appears in a filter selection. */
export const UNCATEGORIZED_SENTINEL = "__uncategorized__";

/** Input shape for the paginated category search command. */
export interface CategorySearchInput {
   /** Empty string → list all active categories ordered by name.
    * Non-empty → prefix-first match, then substring fallback. */
   query: string;
   /** Maximum items to return (default 50, clamped to [1, 500]). */
   limit?: number | null;
}

/** Paginated result from the category search command. */
export interface CategorySearchPage {
   items: CategoryResponse[];
   total: number;
   /** True when `total` exceeds the number of items in `items`. */
   has_more: boolean;
}

/**
 * Paginated category search.
 *
 * - Empty `query`: returns all active categories, ordered by name asc.
 * - Non-empty `query`: prefix-first match (`name ILIKE 'query%'`), falls back
 *   to substring match (`name ILIKE '%query%'`) when no prefix results.
 * - Active categories only (`is_active = 1`); archived categories are excluded.
 * - Respects `limit` (clamped to [1, 500]) with `has_more` flag.
 *
 * @param input.query - Search term (case-insensitive).
 * @param input.limit - Max results to return (default 50, max 500).
 */
export async function listCategoriesSearch(
   input: CategorySearchInput,
): Promise<CategorySearchPage> {
   return invoke<CategorySearchPage>("list_categories_search", { input });
}
