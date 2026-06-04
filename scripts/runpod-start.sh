curl -fsSL https://ollama.com/install.sh | bash
source ~/.bashrc
sleep 5
ollama serve &
sleep 5
ollama pull qwen3.6:27b-mtp-bf16
ollama run qwen3.6:27b-mtp-bf16
# OLLAMA_CONTEXT_LENGTH 256000
# OLLAMA_MODELS /workspace/models
# OLLAMA_KEEP_ALIVE -1