import { json } from "@sveltejs/kit";
import type { RequestHandler } from "@sveltejs/kit";
import { parse } from "path";
import type { BlogPostCardDetails } from "$types";

export const GET: RequestHandler = async () => {
  const modules = import.meta.glob("../posts/**/*+page.md");

  const posts: BlogPostCardDetails[] = [];

  await Promise.all(
    Object.entries(modules).map(async ([file, module]) => {
      const {
        metadata: { title, description, tags, date, icon },
      } = await (module as () => Promise<{ metadata: BlogPostCardDetails & { date: string } }>)();

      posts.push({
        title,
        description,
        tags,
        created: date,
        icon,
        slug: `/blog/${parse(file).dir.split("/").slice(1).join("/")}`,
      });
    }),
  );

  // Newest first
  posts.sort((a, b) => (a.created > b.created ? -1 : 1));

  return json(posts);
};
