import { defineCollection } from "astro:content";
// `z` re-exported from `astro:content` is deprecated; import it from
// `astro/zod` (the pattern nimbus-docs' own schema helpers document).
import { z } from "astro/zod";
import { docsCollection, partialsCollection } from "@cloudflare/nimbus-docs/content";

export const collections = {
  docs: defineCollection(
    docsCollection({
      schemaFields: {
        // Nimbus docs are agent-friendly by default. Set `audience: human`
        // to flag a page that's written primarily for human readers.
        audience: z.literal("human").optional(),
      },
    }),
  ),
  // The Chinese tree. Nimbus mounts the primary `docs` collection at the site
  // root with no prefix and every other collection at `/<collection name>`
  // (see the package's `_internal/collection-mount.ts`), so a collection named
  // `zh-CN` living in `src/content/zh-CN/` is served at `/zh-CN/...` — the same
  // pairing the old docsify site had under `#/zh-CN/...`. Same schema as `docs`.
  "zh-CN": defineCollection(
    docsCollection({
      base: "zh-CN",
      schemaFields: {
        audience: z.literal("human").optional(),
      },
    }),
  ),
  partials: defineCollection(partialsCollection()),
};
