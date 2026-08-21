// Homepage section components.
//
// All functions are plain Rust functions returning `impl IntoView` — no
// `#[component]` macro — and build their HTML with Leptos builder syntax
// (no `view!` macro). Each function renders one section of the homepage.

use leptos::html;
use leptos::prelude::*;

use crate::icons::logos::github::github_icon;
use crate::icons::logos::gmail::gmail_icon;
use crate::icons::logos::linkedin::linkedin_icon;
use crate::icons::logos::npm::npm_icon;
use crate::icons::logos::stackoverflow::stackoverflow_icon;
use crate::icons::logos::svelte::svelte_icon;
use crate::icons::twemoji::flag_india::flag_india_icon;

/// Renders the intro section: waving emoji, name, location (India flag), and
/// links to LinkedIn and GitHub.
pub fn intro_section() -> impl IntoView {
    html::section()
        .attr("class", "text-xl mt-8 border-l-4 pl-8 border-fuchsia-500")
        .child(
            html::p()
                .child(
                    html::span()
                        .attr("role", "img")
                        .attr("aria-label", "Hi")
                        .child("👋"),
                )
                .child(", I'm Saikat!")
                .child(html::br())
                .child("I'm a programmer based out of ")
                .child(
                    html::span()
                        .attr("role", "img")
                        .attr("aria-label", "India")
                        .attr("class", "align-text-bottom")
                        .child(flag_india_icon("inline h-5")),
                )
                .child(html::br())
                .child("I like ")
                .child(
                    html::span()
                        .attr("role", "img")
                        .attr("aria-label", "computers")
                        .child("🖥️"),
                )
                .child(" and ")
                .child(
                    html::span()
                        .attr("role", "img")
                        .attr("aria-label", "gaming")
                        .child("🎮"),
                )
                .child(html::br())
                .child("Connect with me on ")
                .child(
                    html::a()
                        .attr("href", "https://www.linkedin.com/in/saikat-das-13674166/")
                        .attr("target", "_blank")
                        .attr("rel", "noopener noreferrer")
                        .child(
                            html::span()
                                .attr("role", "img")
                                .attr("aria-label", "LinkedIn")
                                .attr("class", "align-text-bottom")
                                .child(linkedin_icon("inline h-5")),
                        ),
                )
                .child(" or ")
                .child(
                    html::a()
                        .attr("href", "https://github.com/saikatdas0790")
                        .attr("target", "_blank")
                        .attr("rel", "noopener noreferrer")
                        .child(
                            html::span()
                                .attr("role", "img")
                                .attr("aria-label", "github")
                                .attr("class", "align-text-bottom")
                                .child(github_icon("inline h-5")),
                        ),
                ),
        )
}

/// Builds a single "Find me at" list item: an `<a>` with an icon and label,
/// styled with a colored left border.
fn find_me_on_link(
    href: &str,
    label: &str,
    border_color: &str,
    icon: impl IntoView,
) -> impl IntoView {
    html::li()
        .attr("class", "mb-4")
        .child(
            html::a()
                .attr("href", href)
                .attr("target", "_blank")
                .attr(
                    "class",
                    format!(
                        "flex gap-2 items-center text-xl p-4 border-l-2 border-{border_color} hover:shadow-md transition duration-300",
                    ),
                )
                .child(icon)
                .child(html::span().child(label)),
        )
}

/// Renders the "Find me at" grid: six social links (Website, GitHub, NPM,
/// Email, LinkedIn, Stack Overflow) in a responsive 2/3-column grid.
pub fn find_me_on_section() -> impl IntoView {
    html::article()
        .child(html::h2().attr("class", "text-3xl my-8 mt-16").child("Find me at"))
        .child(
            html::ul()
                .attr("class", "grid grid-cols-2 md:grid-cols-3")
                .child(find_me_on_link(
                    "https://saikat.dev",
                    "My Website",
                    "yellow-600",
                    svelte_icon("w-5 h-5"),
                ))
                .child(find_me_on_link(
                    "https://github.com/saikatdas0790",
                    "Github",
                    "black",
                    github_icon("w-5 h-5"),
                ))
                .child(find_me_on_link(
                    "https://www.npmjs.com/~saikatdas0790",
                    "NPM",
                    "red-600",
                    npm_icon("w-5 h-5"),
                ))
                .child(find_me_on_link(
                    "mailto:saikatdas0790@gmail.com",
                    "Email",
                    "red-500",
                    gmail_icon("w-5 h-5"),
                ))
                .child(find_me_on_link(
                    "https://www.linkedin.com/in/saikat-das-13674166/",
                    "LinkedIn",
                    "blue-700",
                    linkedin_icon("w-5 h-5"),
                ))
                .child(find_me_on_link(
                    "https://stackoverflow.com/users/3462837/saikat-das",
                    "Stack Overflow",
                    "yellow-500",
                    stackoverflow_icon("w-5 h-5"),
                )),
        )
}

/// Renders the "A bit about me" prose section and the "Orthogonal Skills"
/// subsection. Content is simplified compared to the original Svelte version
/// but structurally complete.
pub fn about_me_section() -> impl IntoView {
    (
        html::h2().attr("class", "text-3xl my-8").child("A bit about me"),
        html::article()
            .attr("class", "prose prose-emerald")
            .child(
                html::p().child(
                    "I'm elated to have discovered SvelteJS last year and since then have \
                     dived headfirst into the SvelteJS, TypeScript and TailwindCSS \
                     ecosystems. Was tinkering with static site generation, ReactJS and \
                     GatsbyJS prior to that.",
                ),
            )
            .child(
                html::p().child(
                    "Have a decent amount of experience with NodeJS / ExpressJS. Have \
                     also gone through the docs on Apollo Server/GraphQL Nexus but \
                     haven't built any GraphQL servers yet. Understand the fundamentals \
                     though, so, shouldn't be difficult at all.",
                ),
            )
            .child(
                html::p().child(
                    "Used Firebase (BaaS) for various projects to rapidly accelerate \
                     initial time to market. Am also quite competent with Google Cloud \
                     as a whole and used their products extensively. Huge proponent of \
                     the entire serverless movement. Google Cloud Run is a goto choice \
                     for all kinds of deployments.",
                ),
            )
            .child(
                html::p().child(
                    "For databases, primarily used PostgreSQL and Firestore. Also used \
                     MongoDB and found it quite intuitive but haven't built any products \
                     on it as MongoDB Atlas costs are quite prohibitive at scale. Not a \
                     huge SQL fan but have working knowledge. Mostly use Prisma 2 to \
                     interact with PostgreSQL. Have also tinkered with Dgraph and \
                     FaunaDB but found them lacking.",
                ),
            )
            .child(
                html::section()
                    .child(
                        "Also quite conversant with CI/CD workflows which include \
                         pushing to a Git repository and that triggering a list of steps \
                         such as:",
                    )
                    .child(
                        html::ul()
                            .child(html::li().child("running DB migrations"))
                            .child(html::li().child("running test suite"))
                            .child(html::li().child("deploying to corresponding environment")),
                    )
                    .child(
                        "Used Google Cloud Build for this primarily. Aware of Github \
                         Actions.",
                    ),
            )
            .child(
                html::p().child(
                    "Decent experience with Figma to be able to quickly do some \
                     wireframes to put thought into form that can be \
                     collaborated/ideated on with a team.",
                ),
            ),
        html::h2().attr("class", "text-3xl my-8").child("Orthogonal Skills"),
        html::article().attr("class", "prose prose-emerald").child(
            html::p().child(
                "Have worked as a product manager before, so I understand what it \
                 takes to build, manage, scale a product and the team behind it. Worked \
                 closely with customer support and on occasion, taken customer \
                 chats/calls to speak directly to end users and get feedback on \
                 products. Managed engineering teams with direct reportees, able to \
                 manage expectations and build rapport.",
            ),
        ),
    )
}

/// Renders the "What I'm Learning" section with a list of learning focuses.
pub fn what_im_learning_section() -> impl IntoView {
    (
        html::h2().attr("class", "text-3xl my-8").child("What I'm Learning"),
        html::article().attr("class", "prose prose-emerald").child(
            html::ul()
                .child(
                    html::li().child(
                        "Test driven development (TDD) especially in the context of \
                         testing frontend and backend apps. Exploring unit tests, \
                         integration tests and end-to-end (e2e) tests. As a library \
                         I'm favouring Jest for unit and integration and Playwright \
                         for end-to-end testing.",
                    ),
                )
                .child(
                    html::li().child(
                        "Blockchains and building decentralized tokenized apps and \
                         smart contracts. I'm favouring Dfinity's Internet Computer \
                         (ICP) for this.",
                    ),
                )
                .child(
                    html::li().child(
                        "WebAssembly and how to use them in web apps. Still \
                         exploring this space.",
                    ),
                ),
        ),
    )
}

/// Renders the "What I'm Working On" section with a paragraph about the
/// Go Bazzinga app and links to its project page and live site.
pub fn what_im_working_on_section() -> impl IntoView {
    (
        html::h2().attr("class", "text-3xl my-8").child("What I'm Working On"),
        html::article()
            .attr("class", "prose prose-emerald")
            .child(
                html::p()
                    .child("I'm currently building the Go Bazzinga app as a \
                            progressive web app (PWA). You can check out what it's \
                            about ")
                    .child(
                        html::a()
                            .attr("href", "/projects/entries/go-bazzinga")
                            .attr("target", "_blank")
                            .child("here"),
                    )
                    .child(" and the app ")
                    .child(
                        html::a()
                            .attr("href", "https://gobazzinga.io")
                            .attr("target", "_blank")
                            .child("here"),
                    ),
            ),
    )
}