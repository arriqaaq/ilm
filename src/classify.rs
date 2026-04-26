use anyhow::Result;

use crate::rag::OllamaClient;

/// Classified intent from a user question.
#[derive(Debug)]
pub enum QueryIntent {
    NarratorInfo {
        name: String,
    },
    NarratorCount {
        name: String,
        book: Option<String>,
    },
    NarratorTeachers {
        name: String,
    },
    NarratorStudents {
        name: String,
    },
    NarratorHadiths {
        name: String,
    },
    IsnadSearch {
        narrators: Vec<String>,
        ordered: bool,
    },
    CommonNarrators {
        name1: String,
        name2: String,
    },
    /// Fallback — use existing semantic RAG.
    ContentQuery,
}

const CLASSIFY_SYSTEM: &str = "\
Classify the user's question about Islamic hadith studies. Output ONLY valid JSON.\n\
Categories:\n\
- {\"intent\":\"narrator_info\",\"name\":\"...\"} - Who is this person, biography, reliability\n\
- {\"intent\":\"narrator_count\",\"name\":\"...\",\"book\":null} - How many hadiths someone narrated. Set book if a specific book is mentioned.\n\
- {\"intent\":\"narrator_teachers\",\"name\":\"...\"} - Who were someone's teachers / who did they hear from\n\
- {\"intent\":\"narrator_students\",\"name\":\"...\"} - Who were someone's students / who heard from them\n\
- {\"intent\":\"narrator_hadiths\",\"name\":\"...\"} - Show hadiths narrated by someone\n\
- {\"intent\":\"isnad_search\",\"narrators\":[\"name1\",\"name2\",...],\"ordered\":false} - Find hadiths narrated through ALL these specific narrators (2 or more). Use for multi-narrator intersection (\"hadiths from Abu Huraira and Nafi\"), transmission chain queries (\"chain between X and Y\"), and ordered chains (\"A from B from C\"). Set ordered=true when the user specifies a sequence/chain order.\n\
- {\"intent\":\"common_narrators\",\"name1\":\"...\",\"name2\":\"...\"} - Find narrators common to two people's chains\n\
- {\"intent\":\"content\"} - General questions about hadith content, meaning, rulings, Quran\n\
\n\
Rules:\n\
- Extract narrator names exactly as the user wrote them.\n\
- If the question is about hadith content or meaning (not about a narrator), always return {\"intent\":\"content\"}.\n\
- Output ONLY the JSON object, nothing else.";

impl OllamaClient {
    /// Classify a user question into a structured intent via a non-streaming LLM call.
    pub async fn classify(
        &self,
        question: &str,
        model_override: Option<&str>,
    ) -> Result<QueryIntent> {
        let json = self
            .chat_json(CLASSIFY_SYSTEM, question, model_override)
            .await?;

        let intent = json["intent"].as_str().unwrap_or("content");

        let result = match intent {
            "narrator_info" => {
                let name = json["name"].as_str().unwrap_or("").to_string();
                if name.is_empty() {
                    QueryIntent::ContentQuery
                } else {
                    QueryIntent::NarratorInfo { name }
                }
            }
            "narrator_count" => {
                let name = json["name"].as_str().unwrap_or("").to_string();
                let book = json["book"].as_str().map(|s| s.to_string());
                if name.is_empty() {
                    QueryIntent::ContentQuery
                } else {
                    QueryIntent::NarratorCount { name, book }
                }
            }
            "narrator_teachers" => {
                let name = json["name"].as_str().unwrap_or("").to_string();
                if name.is_empty() {
                    QueryIntent::ContentQuery
                } else {
                    QueryIntent::NarratorTeachers { name }
                }
            }
            "narrator_students" => {
                let name = json["name"].as_str().unwrap_or("").to_string();
                if name.is_empty() {
                    QueryIntent::ContentQuery
                } else {
                    QueryIntent::NarratorStudents { name }
                }
            }
            "narrator_hadiths" => {
                let name = json["name"].as_str().unwrap_or("").to_string();
                if name.is_empty() {
                    QueryIntent::ContentQuery
                } else {
                    QueryIntent::NarratorHadiths { name }
                }
            }
            "isnad_search" => {
                let narrators: Vec<String> = json["narrators"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let ordered = json["ordered"].as_bool().unwrap_or(false);
                if narrators.len() < 2 {
                    QueryIntent::ContentQuery
                } else {
                    QueryIntent::IsnadSearch { narrators, ordered }
                }
            }
            "common_narrators" => {
                let name1 = json["name1"].as_str().unwrap_or("").to_string();
                let name2 = json["name2"].as_str().unwrap_or("").to_string();
                if name1.is_empty() || name2.is_empty() {
                    QueryIntent::ContentQuery
                } else {
                    QueryIntent::CommonNarrators { name1, name2 }
                }
            }
            _ => QueryIntent::ContentQuery,
        };

        tracing::debug!("Classified question as: {:?}", result);
        Ok(result)
    }
}
