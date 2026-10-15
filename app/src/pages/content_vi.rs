//! The footer pages in Vietnamese: generated from the React site's
//! `src/locales/vi.json` `pages.*` by `scripts/port_strings.py`
//! (verbatim; `{{publisher}}` and `{{contact}}` resolved as
//! `src/components/Pages.tsx` does; `pages.updated` is the catalogue's
//! `pages_updated`). Data, not code: regenerate rather than edit by hand.

use super::types::{AppBlock, Cta, InstagramBlock, PageText, Pages, Section, SiteBlock, Table};

/// Every footer page, in this language.
pub(super) const PAGES: Pages = Pages {
    acknowledgements: PageText {
        title: "Tri ân",
        description: "Ai đã làm ra kinh văn, bản dịch, các bài tương đương và từ điển mà Suttas Map hiển thị, theo giấy phép nào.",
        intro: &[
            "Suttas Map vẽ Kinh tạng Pāli thành một bản đồ. Mọi thứ hiển thị ở đây đều do người khác làm ra và rộng lòng trao tặng: trang này ghi tên tất cả, trong bốn ngôn ngữ mà bản đồ dùng — Pāli, tiếng Anh, tiếng Pháp và tiếng Việt.",
        ],
        app: None,
        sites: &[],
        instagram: None,
        sections: &[
            Section {
                h: "SuttaCentral",
                p: &[
                    "Kinh văn, tên kinh và phần tóm lược, các bộ kinh và các bài tương đương giữa các kinh đều lấy từ [SuttaCentral](https://suttacentral.net) ([sc-data](https://github.com/suttacentral/sc-data), [bilara-data](https://github.com/suttacentral/bilara-data)), công bố theo tuyên bố Creative Commons CC0. Mỗi bản dịch thuộc về dịch giả của nó. Mỗi bài kinh trên bản đồ đều dẫn về SuttaCentral: xin hãy đọc ở đó, và cân nhắc [ủng hộ họ](https://suttacentral.net/donations).",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "Không gian vectơ",
                p: &[
                    "Vị trí trong không gian vectơ được tính từ chính văn bản Pāli, với **Digital Pāḷi Dictionary** của Tỳ-kheo Bodhirasa ([digitalpalidictionary.github.io](https://digitalpalidictionary.github.io), [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/)) để đưa mỗi từ về dạng gốc trong từ điển. Không mục từ nào được hiển thị.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "Với lòng biết ơn",
                p: &[
                    "Xin tri ân từng dịch giả có tên ở trên, Dhamma Society vì ấn bản Mahāsaṅgīti, nhóm SuttaCentral đã xuất bản và gìn giữ kinh văn cùng các bài tương đương, và Tỳ-kheo Bodhirasa vì bộ từ điển.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
        ],
    },
    terms: PageText {
        title: "Điều khoản sử dụng",
        description: "Các điều khoản sử dụng Suttas Map.",
        intro: &[
            "Các điều khoản này áp dụng cho việc bạn sử dụng Suttas Map (“Dịch vụ”), do Thibault Rivière (“Nhà xuất bản”; xem [Pháp lý](/legal)) công bố. Khi sử dụng Dịch vụ, bạn chấp nhận các điều khoản này.",
        ],
        app: None,
        sites: &[],
        instagram: None,
        sections: &[
            Section {
                h: "1. Dịch vụ",
                p: &[
                    "Dịch vụ hiển thị các bài kinh của Kinh tạng Pāli dưới dạng bản đồ: các bài tương đương do SuttaCentral công bố, và một không gian nơi các bài kinh được đặt theo độ tương đồng của văn bản Pāli. Dịch vụ miễn phí, không cần đăng ký và chạy hoàn toàn trong trình duyệt của bạn.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "2. Bản đồ là gì",
                p: &[
                    "Vị trí, liên kết và điểm tương đồng đều được tính toán, không phải là nhận định về kinh văn. Hai bài kinh gần nhau trên bản đồ có thể khác nhau, hai bài xa nhau có thể liên quan: chính kinh văn trên SuttaCentral mới là chuẩn mực duy nhất.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "3. Kinh văn và sở hữu trí tuệ",
                p: &[
                    "Kinh văn do SuttaCentral công bố theo CC0 và thuộc về dịch giả; từ điển dùng để xây dựng không gian vectơ theo giấy phép CC BY-NC 4.0 (xem [Tri ân](/acknowledgements)). Bản thân Dịch vụ — mã nguồn, xử lý dữ liệu, giao diện và tên gọi — thuộc sở hữu của Nhà xuất bản. Bạn được dùng để tham khảo cá nhân, phi thương mại.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "4. Tính sẵn có và trách nhiệm",
                p: &[
                    "Dịch vụ được cung cấp nguyên trạng, không bảo đảm về tính sẵn có hay độ chính xác. Nhà xuất bản có thể thay đổi hoặc ngừng Dịch vụ bất cứ lúc nào.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "5. Luật áp dụng",
                p: &["Các điều khoản này tuân theo luật pháp Pháp."],
                list: &[],
                table: None,
                cta: None,
            },
        ],
    },
    privacy: PageText {
        title: "Quyền riêng tư",
        description: "Chính sách quyền riêng tư của Suttas Map: dữ liệu nào được xử lý, vì sao và trong bao lâu.",
        intro: &[
            "Đây là thông báo về quyền riêng tư theo Quy định chung về bảo vệ dữ liệu (GDPR). Bên kiểm soát dữ liệu là Thibault Rivière, liên hệ riviere.thibault@icloud.com (xem [Pháp lý](/legal)). Suttas Map không có tài khoản, không đặt cookie, không quảng cáo hay theo dõi. Việc tìm kiếm diễn ra trong trình duyệt của bạn: những gì bạn gõ không được gửi đi đâu cả.",
        ],
        app: None,
        sites: &[],
        instagram: None,
        sections: &[
            Section {
                h: "1. Những gì được xử lý",
                p: &[],
                list: &[],
                table: Some(Table {
                    head: &[
                        "Dữ liệu",
                        "Mục đích",
                        "Cơ sở pháp lý (GDPR điều 6)",
                        "Lưu giữ",
                    ],
                    rows: &[&[
                        "Nhật ký kết nối do nhà cung cấp lưu trữ giữ: địa chỉ IP, thời gian, URL được yêu cầu, trình duyệt.",
                        "An ninh, ngăn chặn lạm dụng, duy trì hoạt động của Dịch vụ.",
                        "Lợi ích chính đáng (6.1.f).",
                        "Do nhà cung cấp lưu theo chính sách riêng; thường 30 ngày hoặc ít hơn.",
                    ]],
                }),
                cta: None,
            },
            Section {
                h: "2. Những gì ở lại trong trình duyệt của bạn",
                p: &[
                    "Giao diện và ngôn ngữ, các bộ kinh bạn tô màu hay ẩn, các bảng bạn mở hay đóng được lưu trong bộ nhớ cục bộ của trình duyệt, không bao giờ gửi cho Nhà xuất bản. Xóa dữ liệu trang web trong trình duyệt sẽ xóa chúng.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "3. Quyền của bạn",
                p: &[
                    "Bạn có thể yêu cầu Nhà xuất bản, qua riviere.thibault@icloud.com, truy cập, chỉnh sửa hoặc xóa mọi dữ liệu liên quan đến bạn, và khiếu nại với CNIL ([cnil.fr](https://www.cnil.fr)).",
                ],
                list: &[],
                table: None,
                cta: None,
            },
        ],
    },
    legal: PageText {
        title: "Thông tin pháp lý",
        description: "Nhà xuất bản, lưu trữ và nhãn hiệu của Suttas Map.",
        intro: &[],
        app: None,
        sites: &[],
        instagram: None,
        sections: &[
            Section {
                h: "Nhà xuất bản",
                p: &[
                    "Thibault Rivière. Liên hệ: riviere.thibault@icloud.com. Nhà xuất bản cũng phát hành [PaliCanon](https://palicanon.app) và ứng dụng học Tipiṭaka cho iPhone.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "Lưu trữ",
                p: &[],
                list: &["Cloudflare, Inc., 101 Townsend St, San Francisco, CA 94107, Hoa Kỳ."],
                table: None,
                cta: None,
            },
            Section {
                h: "Nhãn hiệu",
                p: &[
                    "App Store là nhãn hiệu của Apple Inc.; Instagram của Meta Platforms, Inc.; Ko-fi của Ko-fi Labs Ltd. Không bên nào bảo trợ Dịch vụ.",
                ],
                list: &[],
                table: None,
                cta: None,
            },
            Section {
                h: "Xem thêm",
                p: &[
                    "[Điều khoản sử dụng](/terms) · [Quyền riêng tư](/privacy) · [Tri ân](/acknowledgements)",
                ],
                list: &[],
                table: None,
                cta: None,
            },
        ],
    },
    support: PageText {
        title: "Ủng hộ",
        description: "Chi phí vận hành Suttas Map, PaliCanon và ứng dụng Tipiṭaka, và cách bạn có thể giúp giữ chúng miễn phí.",
        intro: &[
            "Suttas Map, PaliCanon và ứng dụng Tipiṭaka cho iPhone đều miễn phí: không quảng cáo, không tài khoản, không bán thông tin của bạn. Một người duy nhất xây dựng và duy trì chúng.",
        ],
        app: Some(AppBlock {
            h: "Ứng dụng Tipiṭaka",
            tagline: "Toàn bộ Kinh tạng Pāli trên iPhone, dùng ngoại tuyến: Pāli và bản dịch song song, từ điển trên từng chữ, lộ trình học, ghi chú — bằng tiếng Anh, tiếng Pháp và tiếng Việt.",
            cta: "Tải ứng dụng cho iPhone",
            alts: &[
                "Màn hình chính của ứng dụng Tipiṭaka",
                "Đọc kinh, Pāli và bản dịch song song",
                "Tìm trong Kinh tạng theo từ hoặc theo nghĩa",
                "Đồ thị khái niệm nối các đoạn kinh liên quan",
                "Ghi chú trên một đoạn kinh",
            ],
        }),
        sites: &[
            SiteBlock {
                h: "PaliCanon",
                name: "PaliCanon",
                tagline: "Tìm kinh theo từ hoặc theo nghĩa, đọc các đoạn tìm được, và trao đổi với mô hình AI bạn chọn — bằng tiếng Anh, tiếng Pháp và tiếng Việt.",
                cta: "Mở PaliCanon",
                href: "https://palicanon.app",
                alts: &[
                    "Trang chủ PaliCanon",
                    "Kết quả tìm kiếm cho “how to meditate”",
                ],
            },
            SiteBlock {
                h: "Suttas Map",
                name: "Suttas Map",
                tagline: "Kinh tạng Pāli dưới dạng bản đồ: các bài tương đương của SuttaCentral, và toàn bộ Kinh tạng được đặt theo nghĩa của văn bản Pāli.",
                cta: "Mở Suttas Map",
                href: "https://viz.palicanon.app",
                alts: &[
                    "Toàn bộ bản đồ SuttaCentral",
                    "Các bài tương đương của MN 10 trên bản đồ SuttaCentral",
                    "SN 56.11 trong không gian vectơ",
                    "DN 16 và các bài tương đương, bằng tiếng Pháp",
                ],
            },
        ],
        instagram: Some(InstagramBlock {
            h: "Trên Instagram",
            tagline: "Mỗi ngày một bài kinh, kèm một suy ngẫm ngắn.",
            cta: "Theo dõi trên Instagram",
        }),
        sections: &[
            Section {
                h: "Chi phí",
                p: &[],
                list: &[
                    "Máy chủ và mạng lưới phía trước các trang web.",
                    "Tên miền.",
                    "Phí thành viên nhà phát triển Apple để giữ ứng dụng trên App Store.",
                    "Tài nguyên tính toán để xây dựng chỉ mục, bản đồ và bản dịch.",
                    "Thời gian: xây dựng, sửa lỗi, thêm ngôn ngữ.",
                ],
                table: None,
                cta: None,
            },
            Section {
                h: "Cách giúp đỡ",
                p: &[
                    "Một khoản đóng góp trên Ko-fi, một lần hay hằng tháng, sẽ dùng trực tiếp cho các chi phí trên.",
                ],
                list: &[],
                table: None,
                cta: Some(Cta {
                    label: "Ủng hộ trên Ko-fi",
                    href: "https://ko-fi.com/tipitaka",
                }),
            },
            Section {
                h: "Những cách khác",
                p: &[
                    "Giáo pháp xưa nay luôn được trao tặng miễn phí; điều này giúp sự trao tặng được tiếp nối.",
                ],
                list: &[
                    "Chia sẻ một bài kinh bạn tìm thấy với ai đó.",
                    "Đánh giá ứng dụng trên [App Store](https://apps.apple.com/app/apple-store/id6760995639?pt=128678815&ct=PaliWebMapSupport&mt=8).",
                    "Báo lỗi: viết cho riviere.thibault@icloud.com.",
                ],
                table: None,
                cta: None,
            },
        ],
    },
};
