import type { SidebarItem } from "@cloudflare/nimbus-docs/types";

/**
 * The site's two documentation languages.
 *
 * Nimbus mounts the primary `docs` collection at the site root with no prefix
 * and every other collection at `/<collection name>` (see the package's
 * `_internal/collection-mount.ts`), so the Chinese tree is one collection named
 * `zh-CN`, living in `src/content/zh-CN/` and served at `/zh-CN/...`, one page
 * per name in the English tree.
 *
 * Two things Nimbus keeps site-wide need a per-collection answer here:
 *
 *   - the sidebar. `sidebar.items` is one JSON-serialized list and has no
 *     locale dimension (`getSidebar()` only varies the tree for `versions`), so
 *     the config carries both languages' groups and {@link forLanguage} keeps
 *     only the current page's, through `getSidebar`'s documented `transform`.
 *   - `<html lang>`. `NimbusConfig.locale` is one value for the whole site, so
 *     `BaseLayout` reads the page's collection instead; Pagefind, screen
 *     readers and hyphenation all follow that attribute.
 */

/**
 * Where each page collection is mounted, mirroring `collectionMountPrefix()`.
 *
 * Exported because `LanguageSwitch` rebuilds a page's route in the other tree
 * from this same table: where a tree lives is one fact, not two.
 */
export const MOUNT_PREFIX: Record<string, string> = {
  docs: "",
  "zh-CN": "/zh-CN",
};

/** The BCP-47 language tag each page collection is written in. */
const COLLECTION_LOCALE: Record<string, string> = {
  docs: "en",
  "zh-CN": "zh-CN",
};

/** The language tag for a page's collection, falling back to the site locale. */
export function localeFor(collection: string | undefined, fallback: string): string {
  return (collection && COLLECTION_LOCALE[collection]) || fallback;
}

function isInLanguage(href: string, prefix: string): boolean {
  if (prefix === "") return href !== "/zh-CN" && !href.startsWith("/zh-CN/");
  return href === prefix || href.startsWith(`${prefix}/`);
}

/** Every in-site destination a sidebar item owns, subtree included. */
function subtreeHrefs(item: SidebarItem): string[] {
  if (item.type === "link") return [item.href];
  if (item.type !== "group") return []; // external links are off-site by definition
  const own = [item.indexHref, item._prefix].filter(
    (href): href is string => typeof href === "string",
  );
  return [...own, ...item.children.flatMap(subtreeHrefs)];
}

/** Keep the top-level groups and links that belong to `collection`'s language. */
export function forLanguage(
  tree: SidebarItem[],
  collection: string = "docs",
): SidebarItem[] {
  const prefix = MOUNT_PREFIX[collection] ?? `/${collection}`;
  return tree.filter((item) =>
    subtreeHrefs(item).some((href) => isInLanguage(href, prefix)),
  );
}
