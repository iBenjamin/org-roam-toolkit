//! HTML rendering with maud (compile-time HTML DSL).

use maud::{html, Markup, DOCTYPE};

pub fn page() -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "ortk-roam-graph" }
                link rel="stylesheet" href="/assets/style.css";
                script src="/assets/graphology.umd.min.js" {}
                script src="/assets/sigma.min.js" {}
                script src="/assets/fa2.worker.js" {}
                script type="module" src="/assets/app.js" {}
            }
            body {
                header.topbar {
                    h1 { "ortk-roam-graph" }
                    div.search {
                        input #search type="text" placeholder="search title or alias…" autocomplete="off";
                        ul #search-results.results {}
                    }
                    div.stats #stats { "—" }
                }
                main.split {
                    section.left {
                        div.tags #tags {}
                        div #sigma {}
                    }
                    div.splitter {}
                    section.right #note {
                        p.placeholder { "click a node" }
                    }
                }
            }
        }
    }
}
