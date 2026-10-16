//! The footer pages in French: generated from the React site's
//! `src/locales/fr.json` `pages.*` by `scripts/port_strings.py`
//! (verbatim; `{{publisher}}` and `{{contact}}` resolved as
//! `src/components/Pages.tsx` does; the hosts `viz.*` rewritten `map.*`;
//! `pages.updated` is the catalogue's `pages_updated`). Data, not code:
//! regenerate rather than edit by hand.

use super::types::{AppBlock, Cta, InstagramBlock, PageText, Pages, Section, SiteBlock, Table};

/// Every footer page, in this language.
pub(super) const PAGES: Pages = Pages {
    acknowledgements: PageText {
        title: "Remerciements",
        description: "Qui a fait les textes, traductions, parallèles et le dictionnaire qu’affiche Suttas Map, et sous quelle licence.",
        intro: &[
            "Suttas Map dessine le Canon pāli comme une carte. Tout ce qu’elle montre a été réalisé par d’autres et offert : cette page les nomme tous, dans les quatre langues qu’utilise la carte — pāli, anglais, français et vietnamien.",
        ],
        app: None,
        sites: &[],
        instagram: None,
        sections: &[
            Section {
                h: "SuttaCentral",
                p: &[
                    "Les textes, leurs titres et résumés, les collections et les parallèles entre textes viennent tous de [SuttaCentral](https://suttacentral.net) ([sc-data](https://github.com/suttacentral/sc-data), [bilara-data](https://github.com/suttacentral/bilara-data)), publiés sous la dédicace Creative Commons CC0. Chaque traduction reste celle de son traducteur. Chaque texte de la carte renvoie à SuttaCentral : lisez-le là-bas, et pensez à [les soutenir](https://suttacentral.net/donations).",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "L’espace vectoriel",
                p: &[
                    "Les positions de l’espace vectoriel sont calculées à partir du texte pāli lui-même, avec le **Digital Pāḷi Dictionary** de Bhikkhu Bodhirasa ([digitalpalidictionary.github.io](https://digitalpalidictionary.github.io), [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/)) pour ramener chaque mot à sa forme de dictionnaire. Aucune entrée du dictionnaire n’est affichée.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "Avec gratitude",
                p: &[
                    "À chacun des traducteurs nommés ci-dessus, à la Dhamma Society pour l’édition Mahāsaṅgīti, à l’équipe de SuttaCentral pour la publication et l’entretien des textes et de leurs parallèles, et à Bhikkhu Bodhirasa pour le dictionnaire.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
        ],
    },
    terms: PageText {
        title: "Conditions d’utilisation",
        description: "Les conditions d’utilisation de Suttas Map.",
        intro: &[
            "Ces conditions régissent votre utilisation de Suttas Map (le « Service »), édité par Thibault Rivière (l’« Éditeur » ; voir [Mentions légales](/legal)). En utilisant le Service, vous les acceptez.",
        ],
        app: None,
        sites: &[],
        instagram: None,
        sections: &[
            Section {
                h: "1. Le Service",
                p: &[
                    "Le Service montre les textes du Canon pāli sous forme de carte : les parallèles publiés par SuttaCentral, et un espace où les textes sont placés selon la similarité de leur pāli. Il est gratuit, sans inscription, et fonctionne entièrement dans votre navigateur.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "2. Ce qu’est la carte",
                p: &[
                    "Les positions, les liens et les scores de similarité sont calculés : ce ne sont pas des jugements sur les textes. Deux textes proches sur la carte peuvent différer, et deux textes éloignés être liés : les textes eux-mêmes, sur SuttaCentral, restent la seule référence.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "3. Textes et propriété intellectuelle",
                p: &[
                    "Les textes sont publiés par SuttaCentral sous CC0 et restent ceux de leurs traducteurs ; le dictionnaire utilisé pour construire l’espace vectoriel est sous CC BY-NC 4.0 (voir [Remerciements](/acknowledgements)). Le Service lui-même — son code, ses traitements de données, son interface et son nom — est la propriété de l’Éditeur. Vous pouvez l’utiliser pour une consultation personnelle et non commerciale.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "4. Disponibilité et responsabilité",
                p: &[
                    "Le Service est fourni en l’état, sans garantie de disponibilité ni d’exactitude. L’Éditeur peut le modifier ou l’arrêter à tout moment.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "5. Droit applicable",
                p: &["Ces conditions sont régies par le droit français."],
                list: &[],
                table: None,
                cta: None,
            },
        ],
    },
    privacy: PageText {
        title: "Confidentialité",
        description: "La politique de confidentialité de Suttas Map : quelles données, pourquoi, et pour combien de temps.",
        intro: &[
            "Ceci est l’information requise par le Règlement général sur la protection des données (RGPD). Le responsable du traitement est Thibault Rivière, contact riviere.thibault@icloud.com (voir [Mentions légales](/legal)). Suttas Map n’a pas de compte, ne dépose aucun cookie et n’utilise ni publicité ni pistage. La recherche se fait dans votre navigateur : ce que vous tapez n’est envoyé nulle part.",
        ],
        app: None,
        sites: &[],
        instagram: None,
        sections: &[
            Section {
                h: "1. Ce qui est traité",
                p: &[],
                list: &[],
                table: Some(Table {
                    head: &[
                        "Données",
                        "Finalité",
                        "Base légale (RGPD art. 6)",
                        "Conservation",
                    ],
                    rows: &[&[
                        "Journaux de connexion tenus par l’hébergeur : adresse IP, heure, URL demandée, navigateur.",
                        "Sécurité, prévention des abus, bon fonctionnement du Service.",
                        "Intérêt légitime (6.1.f).",
                        "Par l’hébergeur, selon sa propre politique ; en général 30 jours ou moins.",
                    ]],
                }),
                cta: None,
            },
            Section {
                h: "2. Ce qui reste dans votre navigateur",
                p: &[
                    "Votre thème et vos langues, les collections colorées ou masquées, les panneaux ouverts ou fermés sont enregistrés dans le stockage local de votre navigateur, jamais envoyés à l’Éditeur. Effacer les données du site dans votre navigateur les supprime.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "3. Vos droits",
                p: &[
                    "Vous pouvez demander à l’Éditeur, à riviere.thibault@icloud.com, l’accès, la rectification ou l’effacement de toute donnée vous concernant, et saisir la CNIL ([cnil.fr](https://www.cnil.fr)).",
                ],
                list: &[],
                table: None,
                cta: None,
            },
        ],
    },
    legal: PageText {
        title: "Mentions légales",
        description: "Éditeur, hébergement et marques de Suttas Map.",
        intro: &[],
        app: None,
        sites: &[],
        instagram: None,
        sections: &[
            Section {
                h: "Éditeur",
                p: &[
                    "Thibault Rivière. Contact : riviere.thibault@icloud.com. L’Éditeur publie aussi [PaliCanon](https://palicanon.app) et l’app d’étude Tipiṭaka pour iPhone.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "Hébergement",
                p: &[],
                list: &["Cloudflare, Inc., 101 Townsend St, San Francisco, CA 94107, États-Unis."],
                table: None,
                cta: None,
            },
            Section {
                h: "Marques",
                p: &[
                    "App Store est une marque d’Apple Inc. ; Instagram de Meta Platforms, Inc. ; Ko-fi de Ko-fi Labs Ltd. Aucune d’elles ne cautionne le Service.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "Voir aussi",
                p: &[
                    "[Conditions d’utilisation](/terms) · [Confidentialité](/privacy) · [Remerciements](/acknowledgements)",
                ],
                list: &[],
                table: None,
                cta: None,
            },
        ],
    },
    support: PageText {
        title: "Soutenir",
        description: "Ce que coûtent Suttas Map, PaliCanon et l’app Tipiṭaka, et comment aider à les garder gratuits.",
        intro: &[
            "Suttas Map, PaliCanon et l’app Tipiṭaka pour iPhone sont gratuits : pas de publicité, pas de compte, rien de vous n’est à vendre. Une seule personne les construit et les fait tourner.",
        ],
        app: Some(AppBlock {
            h: "L’app Tipiṭaka",
            tagline: "Tout le Canon pāli sur votre iPhone, hors ligne : pāli et traductions côte à côte, un dictionnaire sur chaque mot, des parcours d’étude, des notes — en anglais, français et vietnamien.",
            cta: "Télécharger l’app iPhone",
            alts: &[
                "Écran d’accueil de l’app Tipiṭaka",
                "Lecture d’un sutta, pāli et traduction côte à côte",
                "Recherche dans le Canon par mot ou par sens",
                "Graphe de concepts reliant des passages",
                "Prise de notes sur un passage",
            ],
        }),
        sites: &[
            SiteBlock {
                h: "PaliCanon",
                name: "PaliCanon",
                tagline: "Chercher dans les suttas par mots ou par sens, lire les passages trouvés, et en discuter avec le modèle d’IA de votre choix — en anglais, français et vietnamien.",
                cta: "Ouvrir PaliCanon",
                href: "https://palicanon.app",
                alts: &[
                    "Page d’accueil de PaliCanon",
                    "Résultats de recherche pour « how to meditate »",
                ],
            },
            SiteBlock {
                h: "Suttas Map",
                name: "Suttas Map",
                tagline: "Le Canon pāli comme une carte : les parallèles de SuttaCentral entre les textes, et tout le canon placé selon le sens de son pāli.",
                cta: "Ouvrir Suttas Map",
                href: "https://map.palicanon.app",
                alts: &[
                    "La carte SuttaCentral en entier",
                    "Les parallèles de MN 10 sur la carte SuttaCentral",
                    "SN 56.11 dans l’espace vectoriel",
                    "DN 16 et ses parallèles, en français",
                ],
            },
        ],
        instagram: Some(InstagramBlock {
            h: "Sur Instagram",
            tagline: "Un sutta par jour, avec une courte réflexion.",
            cta: "Suivre sur Instagram",
        }),
        sections: &[
            Section {
                h: "Ce que ça coûte",
                p: &[],
                list: &[
                    "Les serveurs et le réseau devant les sites.",
                    "Les noms de domaine.",
                    "L’abonnement développeur Apple qui garde l’app sur l’App Store.",
                    "Le calcul pour construire les index, les cartes et les traductions.",
                    "Le temps : construire, corriger, ajouter des langues.",
                ],
                table: None,
                cta: None,
            },
            Section {
                h: "Comment aider",
                p: &["Un don sur Ko-fi, ponctuel ou mensuel, va directement aux coûts ci-dessus."],
                list: &[],
                table: None,
                cta: Some(Cta {
                    label: "Soutenir sur Ko-fi",
                    href: "https://ko-fi.com/tipitaka",
                }),
            },
            Section {
                h: "Autres façons d’aider",
                p: &["Le Dhamma a toujours été offert librement ; ceci prolonge l’offrande."],
                list: &[
                    "Partager un sutta trouvé avec quelqu’un.",
                    "Noter l’app sur l’[App Store](https://apps.apple.com/app/apple-store/id6760995639?pt=128678815&ct=PaliWebMapSupport&mt=8).",
                    "Signaler une erreur : écrire à riviere.thibault@icloud.com.",
                ],
                table: None,
                cta: None,
            },
        ],
    },
};
