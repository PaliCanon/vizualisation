//! The footer pages in English: generated from the React site's
//! `src/locales/en.json` `pages.*` by `scripts/port_strings.py`
//! (verbatim; `{{publisher}}` and `{{contact}}` resolved as
//! `src/components/Pages.tsx` does; `pages.updated` is the catalogue's
//! `pages_updated`). Data, not code: regenerate rather than edit by hand.

use super::types::{AppBlock, Cta, InstagramBlock, PageText, Pages, Section, SiteBlock, Table};

/// Every footer page, in this language.
pub(super) const PAGES: Pages = Pages {
    acknowledgements: PageText {
        title: "Acknowledgements",
        description: "Who made the texts, translations, parallels and dictionary that Suttas Map shows, and under what licence.",
        intro: &[
            "Suttas Map draws the Pāli Canon as a map. Everything it shows was made by others and given away: this page names them all, in the four languages the map uses — Pāli, English, French and Vietnamese.",
        ],
        app: None,
        sites: &[],
        instagram: None,
        sections: &[
            Section {
                h: "SuttaCentral",
                p: &[
                    "The texts, their titles and summaries, the collections and the parallels between texts all come from [SuttaCentral](https://suttacentral.net) ([sc-data](https://github.com/suttacentral/sc-data), [bilara-data](https://github.com/suttacentral/bilara-data)), published under the Creative Commons CC0 dedication. Each translation remains its translator's. Every text on the map links back to SuttaCentral: please read it there, and consider [supporting them](https://suttacentral.net/donations).",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "The vectorial space",
                p: &[
                    "The positions in the vectorial space are computed from the Pāli text itself, with the **Digital Pāḷi Dictionary** by Bhikkhu Bodhirasa ([digitalpalidictionary.github.io](https://digitalpalidictionary.github.io), [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/)) to bring every word back to its dictionary form. No dictionary entry is shown.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "With gratitude",
                p: &[
                    "To every translator named above, to the Dhamma Society for the Mahāsaṅgīti edition, to the SuttaCentral team for publishing and maintaining the texts and their parallels, and to Bhikkhu Bodhirasa for the dictionary.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
        ],
    },
    terms: PageText {
        title: "Terms of use",
        description: "The terms governing use of Suttas Map.",
        intro: &[
            "These terms govern your use of Suttas Map (the “Service”), published by Thibault Rivière (the “Publisher”; see [Legal](/legal)). By using the Service you accept them.",
        ],
        app: None,
        sites: &[],
        instagram: None,
        sections: &[
            Section {
                h: "1. The Service",
                p: &[
                    "The Service shows the texts of the Pāli Canon as a map: the parallels published by SuttaCentral, and a space where texts are placed by the similarity of their Pāli. It is provided free of charge, without registration, and runs entirely in your browser.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "2. What the map is",
                p: &[
                    "Positions, links and similarity scores are computed, not judgements about the texts. Two texts close on the map may differ, and two distant ones may be related: the texts themselves, on SuttaCentral, remain the only authority.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "3. Texts and intellectual property",
                p: &[
                    "The texts are published by SuttaCentral under CC0 and remain their translators'; the dictionary used to build the vectorial space is under CC BY-NC 4.0 (see [Acknowledgements](/acknowledgements)). The Service itself — its code, data processing, interface and name — is the property of the Publisher. You may use it for personal, non-commercial consultation.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "4. Availability and liability",
                p: &[
                    "The Service is provided as is, without guarantee of availability or accuracy. The Publisher may change or stop it at any time.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "5. Law",
                p: &["These terms are governed by French law."],
                list: &[],
                table: None,
                cta: None,
            },
        ],
    },
    privacy: PageText {
        title: "Privacy",
        description: "Suttas Map's privacy notice: what data is processed, why, and for how long.",
        intro: &[
            "This is the privacy notice required by the General Data Protection Regulation (GDPR). The data controller is Thibault Rivière, contact riviere.thibault@icloud.com (see [Legal](/legal)). Suttas Map has no accounts, sets no cookies, and uses no advertising or tracking. Searches run in your browser: what you type is never sent anywhere.",
        ],
        app: None,
        sites: &[],
        instagram: None,
        sections: &[
            Section {
                h: "1. What is processed",
                p: &[],
                list: &[],
                table: Some(Table {
                    head: &["Data", "Purpose", "Legal basis (GDPR art. 6)", "Kept"],
                    rows: &[&[
                        "Connection logs kept by the hosting provider: IP address, time, requested URL, user agent.",
                        "Security, abuse prevention, keeping the Service running.",
                        "Legitimate interest (6.1.f).",
                        "By the provider, under its own policy; typically 30 days or less.",
                    ]],
                }),
                cta: None,
            },
            Section {
                h: "2. What stays in your browser",
                p: &[
                    "Your theme and languages, the collections you coloured or hid, the panels you opened or closed are stored in your browser's local storage, never sent to the Publisher. Clearing your browser's site data removes them.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "3. Your rights",
                p: &[
                    "You may ask the Publisher at riviere.thibault@icloud.com for access to, correction or deletion of any data concerning you, and complain to the CNIL ([cnil.fr](https://www.cnil.fr)).",
                ],
                list: &[],
                table: None,
                cta: None,
            },
        ],
    },
    legal: PageText {
        title: "Legal notice",
        description: "Publisher, hosting and trademark information for Suttas Map.",
        intro: &[],
        app: None,
        sites: &[],
        instagram: None,
        sections: &[
            Section {
                h: "Publisher · Éditeur",
                p: &[
                    "Thibault Rivière. Contact: riviere.thibault@icloud.com. The Publisher also publishes [PaliCanon](https://palicanon.app) and the Tipiṭaka study app for iPhone.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "Hosting · Hébergement",
                p: &[],
                list: &["Cloudflare, Inc., 101 Townsend St, San Francisco, CA 94107, USA."],
                table: None,
                cta: None,
            },
            Section {
                h: "Trademarks",
                p: &[
                    "App Store is a trademark of Apple Inc.; Instagram of Meta Platforms, Inc.; Ko-fi of Ko-fi Labs Ltd. None of them endorses the Service.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "See also",
                p: &[
                    "[Terms of use](/terms) · [Privacy](/privacy) · [Acknowledgements](/acknowledgements)",
                ],
                list: &[],
                table: None,
                cta: None,
            },
        ],
    },
    support: PageText {
        title: "Support",
        description: "What it costs to run Suttas Map, PaliCanon and the Tipiṭaka app, and how you can help keep them free.",
        intro: &[
            "Suttas Map, PaliCanon and the Tipiṭaka app for iPhone are free: no ads, no accounts, nothing about you for sale. One person builds them and keeps them running.",
        ],
        app: Some(AppBlock {
            h: "The Tipiṭaka app",
            tagline: "The whole Pāli Canon on your iPhone, offline: Pāli and translations side by side, a dictionary on every word, study plans, notes — in English, French and Vietnamese.",
            cta: "Get the iPhone app",
            alts: &[
                "Home screen of the Tipiṭaka app",
                "Reading a sutta, Pāli and translation side by side",
                "Searching the Canon by word or by meaning",
                "Concept graph linking related passages",
                "Taking notes on a passage",
            ],
        }),
        sites: &[
            SiteBlock {
                h: "PaliCanon",
                name: "PaliCanon",
                tagline: "Search the suttas by words or by meaning, read the passages found, and discuss them with an AI model of your choice — in English, French and Vietnamese.",
                cta: "Open PaliCanon",
                href: "https://palicanon.app",
                alts: &[
                    "PaliCanon home page",
                    "Search results for “how to meditate”",
                ],
            },
            SiteBlock {
                h: "Suttas Map",
                name: "Suttas Map",
                tagline: "The Pāli Canon as a map: SuttaCentral’s parallels between texts, and the whole canon placed by the meaning of its Pāli.",
                cta: "Open Suttas Map",
                href: "https://viz.palicanon.app",
                alts: &[
                    "The whole SuttaCentral map",
                    "The parallels of MN 10 on the SuttaCentral map",
                    "SN 56.11 in the vectorial space",
                    "DN 16 and its parallels, in French",
                ],
            },
        ],
        instagram: Some(InstagramBlock {
            h: "On Instagram",
            tagline: "A sutta a day, with a short reflection.",
            cta: "Follow on Instagram",
        }),
        sections: &[
            Section {
                h: "What it costs",
                p: &[],
                list: &[
                    "The servers and the network in front of the sites.",
                    "The domain names.",
                    "The Apple developer membership that keeps the app on the App Store.",
                    "The computing to build the indexes, the maps and the translations.",
                    "Time: building, fixing, adding languages.",
                ],
                table: None,
                cta: None,
            },
            Section {
                h: "How to help",
                p: &[
                    "A donation on Ko-fi, one-off or monthly, goes straight toward the costs above.",
                ],
                list: &[],
                table: None,
                cta: Some(Cta {
                    label: "Support on Ko-fi",
                    href: "https://ko-fi.com/tipitaka",
                }),
            },
            Section {
                h: "Other ways to help",
                p: &["The Dhamma has always been offered freely; this keeps the offering going."],
                list: &[
                    "Share a sutta you found with someone.",
                    "Rate the app on the [App Store](https://apps.apple.com/app/apple-store/id6760995639?pt=128678815&ct=PaliWebMapSupport&mt=8).",
                    "Report a mistake: write to riviere.thibault@icloud.com.",
                ],
                table: None,
                cta: None,
            },
        ],
    },
};
