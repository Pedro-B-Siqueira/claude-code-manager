import { mount } from 'svelte';
import App from './App.svelte';
import { PLATFORM } from './lib/platform';
import './lib/theme/base.css';

document.documentElement.dataset.platform = PLATFORM;

const target = document.getElementById('app');

if (target) {
  mount(App, { target });
}

export {};
