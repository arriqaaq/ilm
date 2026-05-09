#!/usr/bin/env python3
"""
Fetch all hadith-grading books from turath.io.

Books here are the source of truth for per-hadith multi-scholar verdicts
(Albani Sunan series, Silsilatān, Jami al-Saghir, Daraqutni Ilal,
Talkhis al-Habir) and a canonical Sunan source (Ibn Majah, ed. Hadi).

All IDs were verified directly against turath.io by sampling page text.
Books not findable as standalone items in Turath search (Sahih Sunan al-Tirmidhi,
Sahih/Daif Sunan Ibn Majah, Sahih Sunan Abi Dawud Maarif edition) are NOT here —
their grading is recovered through Jami al-Saghir cross-references in Step B'.

Usage:
  python3 scripts/fetch_grading_books.py            # metadata only
  python3 scripts/fetch_grading_books.py --pages    # full pages

After fetch, the script prints `cargo run -- ingest-turath ...` commands to
load each book into SurrealDB.
"""

import os

from _turath_fetch import run_book

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA_DIR = os.path.join(ROOT, "data")

# (book_id, slug, name_en, name_ar, author_ar, scholar_key, category)
BOOKS = [
    (1147,   "sahih_sunan_nasai",       "Sahih Sunan al-Nasai (Albani)",          "صحيح سنن النسائي",                                     "ناصر الدين الألباني",   "albani",     "hadith_grading"),
    (1148,   "daif_sunan_nasai",        "Daif Sunan al-Nasai (Albani)",           "ضعيف سنن النسائي",                                      "ناصر الدين الألباني",   "albani",     "hadith_grading"),
    (1216,   "daif_sunan_tirmidhi",     "Daif Sunan al-Tirmidhi (Albani)",        "ضعيف سنن الترمذي",                                      "ناصر الدين الألباني",   "albani",     "hadith_grading"),
    (25881,  "sahih_sunan_abi_dawud",   "Sahih Sunan Abi Dawud (Albani, Ghiras)", "صحيح سنن أبي داود ط غراس",                              "ناصر الدين الألباني",   "albani",     "hadith_grading"),
    (5914,   "daif_sunan_abi_dawud",    "Daif Sunan Abi Dawud al-Umm (Albani)",   "ضعيف أبي داود - الأم",                                  "ناصر الدين الألباني",   "albani",     "hadith_grading"),
    (10757,  "sahih_jami_saghir",       "Sahih al-Jami al-Saghir (Albani)",       "صحيح الجامع الصغير وزيادته",                            "ناصر الدين الألباني",   "albani",     "hadith_grading"),
    (1663,   "daif_jami_saghir",        "Daif al-Jami al-Saghir (Albani)",        "ضعيف الجامع الصغير وزيادته",                            "ناصر الدين الألباني",   "albani",     "hadith_grading"),
    (9442,   "silsilat_sahihah",        "Silsilat al-Ahadith al-Sahihah",         "سلسلة الأحاديث الصحيحة وشيء من فقهها وفوائدها",         "ناصر الدين الألباني",   "albani",     "hadith_grading"),
    (12762,  "silsilat_daifah",         "Silsilat al-Ahadith al-Daifah",          "سلسلة الأحاديث الضعيفة والموضوعة وأثرها السيئ في الأمة", "ناصر الدين الألباني",   "albani",     "hadith_grading"),
    (22592,  "irwa_al_ghalil",          "Irwa al-Ghalil",                          "إرواء الغليل في تخريج أحاديث منار السبيل",              "ناصر الدين الألباني",   "albani",     "hadith_grading"),
    (9082,   "ilal_daraqutni",          "Ilal al-Daraqutni",                       "العلل الواردة في الأحاديث النبوية",                      "الدارقطني",            "daraqutni",  "hadith_grading"),
    (123608, "talkhis_habir",           "Talkhis al-Habir (Ibn Hajar)",            "التلخيص الحبير",                                        "ابن حجر العسقلاني",     "ibn_hajar",  "hadith_grading"),
    (1194,   "sunan_ibn_majah",         "Sunan Ibn Majah (source, ed. Hadi)",      "سنن ابن ماجه - ت هادي",                                  "ابن ماجه",             "source",     "hadith_collection"),
]


def shell_quote(s):
    return "'" + s.replace("'", "'\\''") + "'"


def main():
    os.makedirs(DATA_DIR, exist_ok=True)

    # Phase 1: fetch metadata + pages (when --pages is given) for every book.
    for (book_id, slug, name_en, name_ar, author_ar, scholar_key, category) in BOOKS:
        headings = os.path.join(DATA_DIR, f"{slug}_headings.json")
        pages = os.path.join(DATA_DIR, f"{slug}_pages.json")
        print(f"\n=== book_id={book_id}  {name_en} ===")
        try:
            run_book(book_id=book_id, display_name=name_en,
                     headings_path=headings, pages_path=pages)
        except Exception as e:
            print(f"  FAILED: {e}")
            continue

    # Phase 2: print the ingest commands.
    print("\n--- Ingest commands (run from project root) ---")
    for (book_id, slug, name_en, name_ar, author_ar, scholar_key, category) in BOOKS:
        pages = f"data/{slug}_pages.json"
        headings = f"data/{slug}_headings.json"
        print(
            f"cargo run --release -- ingest-turath "
            f"--pages-file={shell_quote(pages)} "
            f"--headings-file={shell_quote(headings)} "
            f"--book-id={book_id} "
            f"--name-ar={shell_quote(name_ar)} "
            f"--name-en={shell_quote(name_en)} "
            f"--author-ar={shell_quote(author_ar)} "
            f"--category={shell_quote(category)} "
            f"--book-type='grading'"
        )


if __name__ == "__main__":
    main()
