export type BlogPostCardDetails = {
  title: string;
  description: string;
  tags: string[];
  created: string;
  icon: string;
  slug: string;
};

export type ProjectEntryCardDetails = {
  title: string;
  description: string;
  technologiesUsed: string[];
  coverPhoto: string;
  startDate: Date;
  endDate: Date;
  slug: string;
};
