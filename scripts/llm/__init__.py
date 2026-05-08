"""Provider-agnostic LLM and embedding client for the Python data-prep scripts.

Mirrors the Rust `crate::llm` and `crate::embedding` shapes so users can pick the
same provider on both sides. Built on litellm for chat/embeddings.
"""

from .provider import (
    ChatOptions,
    EmbeddingProvider,
    LiteLlmEmbedder,
    LiteLlmProvider,
    LlmProvider,
    TokenEvent,
    build_embedder,
    build_llm,
)

__all__ = [
    "ChatOptions",
    "EmbeddingProvider",
    "LiteLlmEmbedder",
    "LiteLlmProvider",
    "LlmProvider",
    "TokenEvent",
    "build_embedder",
    "build_llm",
]
