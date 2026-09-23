import js from '@eslint/js';
import globals from 'globals';

export default [
  { ignores: ['node_modules/**', 'dist/**'] },
  js.configs.recommended,
  {
    files: ['core/assets/**/*.js'],
    languageOptions: { globals: globals.browser },
    rules: { 'no-unused-vars': ['error', { caughtErrors: 'none' }] },
  },
  {
    files: ['tests/frontend/**/*.cjs', '*.mjs'],
    languageOptions: { globals: globals.node },
  },
];
