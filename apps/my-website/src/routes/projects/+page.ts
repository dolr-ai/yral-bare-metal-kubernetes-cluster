import type { ProjectEntryCardDetails } from "$types";

export const load = async ({ fetch }: { fetch: typeof globalThis.fetch }) => {
    const response = await fetch("/projects/getProjectEntries.json");
    const projectEntries: ProjectEntryCardDetails[] = await response.json();
    return { projectEntries };
};
