"""LLM and embedding provider abstraction for offline data-prep scripts.

Two protocol classes (`LlmProvider`, `EmbeddingProvider`) plus litellm-backed
default adapters. Custom adapters need only satisfy the protocol — no litellm
dependency required.

litellm encodes the provider in the model string: `ollama/llama3.2`,
`openai/gpt-4o-mini`, `anthropic/claude-opus-4-7`, `voyage/voyage-3`, etc.
"""

from __future__ import annotations

import os
from dataclasses import dataclass
from typing import Iterator, List, Optional, Protocol


@dataclass
class ChatOptions:
    """Per-call chat options. Pass to `LlmProvider.chat_*` methods."""

    model: Optional[str] = None
    temperature: float = 0.7
    max_tokens: Optional[int] = None
    response_format: Optional[str] = None  # "json" | None


@dataclass
class TokenEvent:
    delta: str
    done: bool


class LlmProvider(Protocol):
    """Protocol any chat provider must satisfy."""

    @property
    def default_model(self) -> str: ...

    @property
    def provider_name(self) -> str: ...

    def chat_text(self, system: str, user: str, opts: ChatOptions) -> str:
        """Non-streaming text completion."""
        ...

    def chat_json(self, system: str, user: str, opts: ChatOptions) -> dict:
        """Non-streaming JSON-mode completion. Returns parsed dict."""
        ...

    def chat_stream(
        self, system: str, user: str, opts: ChatOptions
    ) -> Iterator[TokenEvent]:
        """Streaming text completion as an iterator of TokenEvents."""
        ...


class EmbeddingProvider(Protocol):
    """Protocol any embedding provider must satisfy."""

    @property
    def dimension(self) -> int: ...

    @property
    def provider_name(self) -> str: ...

    @property
    def model_name(self) -> str: ...

    def embed_passages(self, texts: List[str]) -> List[List[float]]: ...

    def embed_query(self, text: str) -> List[float]: ...


# ── litellm-backed default adapters ──────────────────────────────────────────


class LiteLlmProvider:
    """Default LlmProvider backed by litellm.completion. Works with any provider
    litellm supports (ollama/openai/anthropic/...).
    """

    def __init__(
        self,
        provider: str,
        model: str,
        api_key: Optional[str] = None,
        base_url: Optional[str] = None,
    ) -> None:
        # Import lazily so adapters that don't use litellm aren't forced to
        # install it.
        try:
            import litellm  # noqa: F401
        except ImportError as e:
            raise ImportError(
                "litellm is required for the default LlmProvider. "
                "Install with `pip install litellm`."
            ) from e

        self._provider = provider
        self._model = model
        self._api_key = api_key
        self._base_url = base_url
        # Compose litellm's expected `provider/model` string.
        self._model_string = f"{provider}/{model}"

    @property
    def default_model(self) -> str:
        return self._model

    @property
    def provider_name(self) -> str:
        return self._provider

    def _common_kwargs(self, opts: ChatOptions) -> dict:
        kwargs: dict = {
            "model": opts.model and f"{self._provider}/{opts.model}" or self._model_string,
            "temperature": opts.temperature,
            "num_retries": 0,
        }
        if opts.max_tokens is not None:
            kwargs["max_tokens"] = opts.max_tokens
        if self._api_key is not None:
            kwargs["api_key"] = self._api_key
        if self._base_url is not None:
            # Ollama uses `api_base` to point at a custom server.
            kwargs["api_base"] = self._base_url
        if opts.response_format == "json":
            kwargs["response_format"] = {"type": "json_object"}
        return kwargs

    def chat_text(self, system: str, user: str, opts: ChatOptions) -> str:
        import litellm

        resp = litellm.completion(
            messages=[
                {"role": "system", "content": system},
                {"role": "user", "content": user},
            ],
            stream=False,
            **self._common_kwargs(opts),
        )
        return resp.choices[0].message.content or ""

    def chat_json(self, system: str, user: str, opts: ChatOptions) -> dict:
        import json
        import re

        json_opts = ChatOptions(
            model=opts.model,
            temperature=opts.temperature,
            max_tokens=opts.max_tokens,
            response_format="json",
        )
        text = self.chat_text(system, user, json_opts)
        try:
            return json.loads(text)
        except json.JSONDecodeError:
            # Some providers (Anthropic without explicit JSON mode) wrap output
            # in prose or fences. Try to recover the first {...} blob.
            m = re.search(r"\{.*\}", text, re.DOTALL)
            if m:
                try:
                    return json.loads(m.group(0))
                except json.JSONDecodeError:
                    pass
            raise

    def chat_stream(
        self, system: str, user: str, opts: ChatOptions
    ) -> Iterator[TokenEvent]:
        import litellm

        stream = litellm.completion(
            messages=[
                {"role": "system", "content": system},
                {"role": "user", "content": user},
            ],
            stream=True,
            **self._common_kwargs(opts),
        )
        for chunk in stream:
            choices = getattr(chunk, "choices", None) or []
            if not choices:
                continue
            delta = getattr(choices[0], "delta", None)
            content = getattr(delta, "content", None) if delta is not None else None
            finish = getattr(choices[0], "finish_reason", None)
            if content:
                yield TokenEvent(delta=content, done=False)
            if finish:
                yield TokenEvent(delta="", done=True)
                return


class LiteLlmEmbedder:
    """Default EmbeddingProvider backed by litellm.embedding."""

    def __init__(
        self,
        provider: str,
        model: str,
        dimension: Optional[int] = None,
        api_key: Optional[str] = None,
        base_url: Optional[str] = None,
    ) -> None:
        try:
            import litellm  # noqa: F401
        except ImportError as e:
            raise ImportError(
                "litellm is required for the default EmbeddingProvider. "
                "Install with `pip install litellm`."
            ) from e

        self._provider = provider
        self._model = model
        self._model_string = f"{provider}/{model}"
        self._api_key = api_key
        self._base_url = base_url
        self._dimension = dimension or _default_dimension(provider, model)

    @property
    def dimension(self) -> int:
        return self._dimension

    @property
    def provider_name(self) -> str:
        return self._provider

    @property
    def model_name(self) -> str:
        return self._model

    def _kwargs(self) -> dict:
        kwargs: dict = {"model": self._model_string}
        if self._api_key is not None:
            kwargs["api_key"] = self._api_key
        if self._base_url is not None:
            kwargs["api_base"] = self._base_url
        return kwargs

    def embed_passages(self, texts: List[str]) -> List[List[float]]:
        import litellm

        if not texts:
            return []
        resp = litellm.embedding(input=list(texts), **self._kwargs())
        return [d["embedding"] for d in resp.data]

    def embed_query(self, text: str) -> List[float]:
        return self.embed_passages([text])[0]


def _default_dimension(provider: str, model: str) -> int:
    table = {
        ("openai", "text-embedding-3-small"): 1536,
        ("openai", "text-embedding-3-large"): 3072,
        ("openai", "text-embedding-ada-002"): 1536,
        ("fastembed", "bge-m3"): 1024,
        ("fastembed", "e5-small"): 384,
    }
    return table.get((provider, model), 0)


# ── Factory helpers ──────────────────────────────────────────────────────────


def build_llm(
    *,
    provider: Optional[str] = None,
    model: Optional[str] = None,
    api_key: Optional[str] = None,
    base_url: Optional[str] = None,
) -> LlmProvider:
    """Build an LlmProvider from explicit args or LLM_* env vars.

    Args fall back to LLM_PROVIDER, LLM_MODEL, LLM_API_KEY, LLM_BASE_URL.
    """

    provider = provider or os.environ.get("LLM_PROVIDER", "ollama")
    model = model or os.environ.get("LLM_MODEL")
    if not model:
        raise ValueError(
            "LLM model is required (pass model= or set LLM_MODEL env)."
        )
    api_key = api_key or os.environ.get("LLM_API_KEY")
    base_url = base_url or os.environ.get("LLM_BASE_URL")
    return LiteLlmProvider(
        provider=provider, model=model, api_key=api_key, base_url=base_url
    )


def build_embedder(
    *,
    provider: Optional[str] = None,
    model: Optional[str] = None,
    api_key: Optional[str] = None,
    base_url: Optional[str] = None,
    dimension: Optional[int] = None,
) -> EmbeddingProvider:
    """Build an EmbeddingProvider from explicit args or EMBED_* env vars."""

    provider = provider or os.environ.get("EMBED_PROVIDER", "openai")
    model = model or os.environ.get("EMBED_MODEL")
    if not model:
        raise ValueError(
            "Embedding model is required (pass model= or set EMBED_MODEL env)."
        )
    api_key = api_key or os.environ.get("EMBED_API_KEY")
    base_url = base_url or os.environ.get("EMBED_BASE_URL")
    if dimension is None and os.environ.get("EMBED_DIMENSIONS"):
        dimension = int(os.environ["EMBED_DIMENSIONS"])
    return LiteLlmEmbedder(
        provider=provider,
        model=model,
        api_key=api_key,
        base_url=base_url,
        dimension=dimension,
    )
