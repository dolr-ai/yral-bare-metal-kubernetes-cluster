import { json } from "@sveltejs/kit";
import type { RequestHandler } from "@sveltejs/kit";
import { parse } from "path";
import type { ProjectEntryCardDetails } from "$types";

export const GET: RequestHandler = async () => {
  const modules = import.meta.glob("../entries/**/*+page.md");

  const entries: ProjectEntryCardDetails[] = [];

  await Promise.all(
    Object.entries(modules).map(async ([file, module]) => {
      const {
        metadata: {
          title,
          description,
          technologiesUsed,
          startDate,
          endDate,
          coverPhoto,
        },
      } = await (module as () => Promise<{ metadata: ProjectEntryCardDetails & { startDate: string; endDate: string } }>)();

      entries.push({
        title,
        description,
        technologiesUsed,
        startDate: new Date(startDate),
        endDate: new Date(endDate),
        coverPhoto,
        slug: `/projects/${parse(file).dir.split("/").slice(1).join("/")}`,
      });
    }),
  );

  // Newest first
  entries.sort((a, b) => (a.startDate > b.startDate ? -1 : 1));

  return json(entries);
};
