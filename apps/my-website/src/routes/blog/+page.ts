import type { BlogPostCardDetails } from "$types";

export const load = async ({ fetch }: { fetch: typeof globalThis.fetch }) => {
    const response = await fetch("/blog/getBlogPosts.json");
    const blogPosts: BlogPostCardDetails[] = await response.json();
    return { blogPosts };
};
