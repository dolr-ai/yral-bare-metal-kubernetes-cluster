import { json } from "@sveltejs/kit";
import type { RequestHandler } from "@sveltejs/kit";

export const prerender = true;
import { parse } from "path";
import type { BlogPostCardDetails } from "$types";
import type { Index } from "lunr";
import lunr from "lunr";
const { Builder } = lunr;

const getAllBlogPosts = async (): Promise<BlogPostCardDetails[]> => {
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
  return posts;
};

const getSearchIndexFromAllPosts = async (
  posts: BlogPostCardDetails[],
): Promise<Index> => {
  const indexBuilder = new Builder();
  indexBuilder.ref("slug");
  indexBuilder.field("tags", { boost: 10 });
  indexBuilder.field("title", { boost: 8 });
  indexBuilder.field("description", { boost: 5 });

  posts
    .map((post) => ({
      slug: post.slug,
      tags: post.tags.join(" "),
      title: post.title,
      description: post.description,
    }))
    .forEach((post) => indexBuilder.add(post));
  return indexBuilder.build();
};

export const GET: RequestHandler = async () => {
  const posts: BlogPostCardDetails[] = await getAllBlogPosts();
  const searchIndex: Index = await getSearchIndexFromAllPosts(posts);
  return json(searchIndex);
};
