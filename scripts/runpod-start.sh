curl -fsSL https://ollama.com/install.sh | bash;
ollama pull qwen3.6:27b-bf16;
export OLLAMA_CONTEXT_LENGTH=256000;
export OLLAMA_MODELS=/workspace/models;
export OLLAMA_NUM_PARALLEL=4;
export OLLAMA_KEEP_ALIVE=-1;
export OLLAMA_FLASH_ATTENTION=1;