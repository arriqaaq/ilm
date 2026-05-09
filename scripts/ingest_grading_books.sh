#!/usr/bin/env bash
# Ingest the 13 verified grading books (Albani's Sunan series, Silsilatān,
# Jami al-Saghir, Daraqutni Ilal, Talkhis al-Habir, source Sunan Ibn Majah)
# into the `book` / `book_page` tables. Uses the existing `ingest-turath` CLI.
#
# Source of truth for the book list: scripts/fetch_grading_books.py BOOKS.
# Keep this file in sync with that list.
#
# Run via:  make book-ingest-grading-books

set -euo pipefail

CARGO_FEATURES="${CARGO_FEATURES:-}"

ingest() {
  local slug="$1" book_id="$2" name_ar="$3" name_en="$4" author_ar="$5" category="$6"
  local pages="data/${slug}_pages.json"
  local headings="data/${slug}_headings.json"

  if [ ! -f "$pages" ]; then
    echo "  [skip] $slug — $pages not found (run: make turath-fetch-grading first)"
    return 0
  fi

  echo "=== Ingesting book_id=$book_id ($name_en) ==="
  cargo run $CARGO_FEATURES -- ingest-turath \
    --pages-file "$pages" \
    --headings-file "$headings" \
    --book-id "$book_id" \
    --name-ar "$name_ar" \
    --name-en "$name_en" \
    --author-ar "$author_ar" \
    --category "$category" \
    --book-type 'grading'
}

ingest sahih_sunan_nasai      1147   "صحيح سنن النسائي"                                       "Sahih Sunan al-Nasai (Albani)"            "ناصر الدين الألباني"   hadith_grading
ingest daif_sunan_nasai       1148   "ضعيف سنن النسائي"                                        "Daif Sunan al-Nasai (Albani)"             "ناصر الدين الألباني"   hadith_grading
ingest daif_sunan_tirmidhi    1216   "ضعيف سنن الترمذي"                                        "Daif Sunan al-Tirmidhi (Albani)"          "ناصر الدين الألباني"   hadith_grading
ingest sahih_sunan_abi_dawud  25881  "صحيح سنن أبي داود ط غراس"                                 "Sahih Sunan Abi Dawud (Albani, Ghiras)"   "ناصر الدين الألباني"   hadith_grading
ingest daif_sunan_abi_dawud   5914   "ضعيف أبي داود - الأم"                                    "Daif Sunan Abi Dawud al-Umm (Albani)"     "ناصر الدين الألباني"   hadith_grading
ingest sahih_jami_saghir      10757  "صحيح الجامع الصغير وزيادته"                              "Sahih al-Jami al-Saghir (Albani)"         "ناصر الدين الألباني"   hadith_grading
ingest daif_jami_saghir       1663   "ضعيف الجامع الصغير وزيادته"                              "Daif al-Jami al-Saghir (Albani)"          "ناصر الدين الألباني"   hadith_grading
ingest silsilat_sahihah       9442   "سلسلة الأحاديث الصحيحة وشيء من فقهها وفوائدها"           "Silsilat al-Ahadith al-Sahihah"           "ناصر الدين الألباني"   hadith_grading
ingest silsilat_daifah        12762  "سلسلة الأحاديث الضعيفة والموضوعة وأثرها السيئ في الأمة"  "Silsilat al-Ahadith al-Daifah"            "ناصر الدين الألباني"   hadith_grading
ingest irwa_al_ghalil         22592  "إرواء الغليل في تخريج أحاديث منار السبيل"                "Irwa al-Ghalil"                            "ناصر الدين الألباني"   hadith_grading
ingest ilal_daraqutni         9082   "العلل الواردة في الأحاديث النبوية"                        "Ilal al-Daraqutni"                         "الدارقطني"             hadith_grading
ingest talkhis_habir          123608 "التلخيص الحبير"                                          "Talkhis al-Habir (Ibn Hajar)"             "ابن حجر العسقلاني"      hadith_grading
ingest sunan_ibn_majah        1194   "سنن ابن ماجه - ت هادي"                                    "Sunan Ibn Majah (source, ed. Hadi)"        "ابن ماجه"              hadith_collection

echo ""
echo "✓ All available grading books ingested into book / book_page."
