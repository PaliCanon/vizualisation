import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

// Same stack as palisearch/web: React + Tailwind v4.
export default defineConfig({
  plugins: [react(), tailwindcss()],
});
