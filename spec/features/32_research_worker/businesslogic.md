# Logic: 32_research_worker

## Core Logic

### 1. Tools
- **Web Search**: Google/Bing/Perplexity API.
- **Code Search**: `grep`, `ripgrep`, or Vector DB semantic search.
- **Doc Fetcher**: Scrape and markdown-ify documentation URLs.

### 2. Synthesis
- **Process**: Gather N sources -> Summarize -> Answer specific question.

## Data Flow
Question -> ResearchWorker -> Tools -> Aggregation -> Answer
