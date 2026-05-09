# Project Notes

## Data Quality Warnings

### Never ingest narrator grading or bio data from SemanticHadith dataset
- The `semantic_hadith.json` narrator entries contain `grade` fields (e.g. `thiqah`, `matruk`) and biographical info that are **unreliable**
- Known issue: narrator HN05049 (Ibn Abi Shaybah, narrator in Sahih Muslim) is incorrectly graded `matruk` — this is the grandfather's grade applied to the grandson
- **Rule**: Only use SemanticHadith for hadith text, isnad chains, and narrator *names/IDs* for matching purposes
- **For narrator bios and grading**: Use Tahdhib al-Tahdhib (Turath book 1278) or other verified classical sources
- This applies to any future ingestion pipeline — do not store or display SemanticHadith grading data
- **Enforced in code** (Apr 17 2026): `reliability_rating`, `reliability_source`, `ibn_hajar_rank` fields removed from DB schema, models, API responses. `evidence` and `scholarly_source` tables removed. Ingestion in `semantic.rs` skips these fields.

## Per-Hadith Grading Coverage Gaps

The per-hadith grading pipeline (`hadith_grading` table, `/v1/hadiths/{id}/gradings`) draws from Albani's grading books on Turath. Coverage by collection is uneven because not every Sunan has a cleanly-extractable source:

| Collection | Coverage | Source |
|---|---|---|
| Bukhari, Muslim | 100% (synthetic "consensus sahih" row) | API handler injection |
| Sunan an-Nasa'i | ~92% | Albani Sahih + Daif Sunan al-Nasa'i (Turath 1147 + 1148) — clean tabular format |
| Sunan Abi Dawud | ~45% | Albani Sahih + Daif Abu Dawud al-Umm (Turath 25881 + 5914) — capped because Turath only carries some volumes of the Umm series |
| Jami at-Tirmidhi | ~50% | Albani Daif Sunan al-Tirmidhi (1216) + Tirmidhi's own inline gradings extracted from Tuhfat al-Ahwadhi (21662) via the existing sharh-mapping (positional alignment when multiple hadiths share a page) |
| Sunan Ibn Majah | **0%** | No standalone Albani Sahih/Daif Sunan Ibn Majah in Turath — searches under multiple spellings returned nothing. **v2 path:** fuzzy matn-match against Sahih/Daif al-Targhib wa al-Tarhib (Turath 171, 179) and Mishkat al-Masabih |

When the API returns `gradings: []` for a hadith, the UI's `<GradingPanel>` correctly hides the panel — no rendering bug.
