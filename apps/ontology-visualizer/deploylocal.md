# Local Deployment Guide for Ontology Visualizer

This document explains how to run the Ontology Visualizer application on your local machine.

## Prerequisites

- Node.js installed on your system.
- NPM (comes with Node.js) for package management.

## Setup Instructions

1. **Navigate to the visualizer directory:**
   ```bash
   cd ./ontology-visualizer
   ```

2. **Install dependencies:**
   This step only needs to be performed once, or when dependencies in `package.json` change.
   ```bash
   npm install
   ```

## Running the Application

There are two primary ways to run the visualizer depending on your needs.

### 1. Development Mode (Recommended for development)

This starts a local development server with Hot Module Replacement (HMR). Any changes made to the source files will instantly be reflected in the browser.

```bash
npm run dev
```
The application will usually be accessible at `http://localhost:5173/`.

### 2. Production Mode (Recommended to verify the final build)

This creates an optimized production bundle and serves it using a lightweight local web server. This is useful for testing exactly what will be deployed to a production environment.

```bash
# Build the production bundle into the 'dist' folder
npm run build

# Serve the built application locally
npm run preview
```
The application will be accessible at `http://localhost:4173/`.

## Troubleshooting

- If you encounter dependency issues, try running `npm clean-install` or removing the `node_modules` folder and running `npm install` again.
- If port `5173` or `4173` is already in use, Vite might automatically assign the next available port (like `5174` or `4174`). Always check the terminal output for the correct URL.
