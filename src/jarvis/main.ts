import { mount } from 'svelte';
import { PLATFORM } from '../lib/platform';
import '../lib/theme/base.css';
import JarvisApp from './JarvisApp.svelte';
import './theme.css';

document.documentElement.dataset.platform = PLATFORM;

const target = document.getElementById('jarvis');

if (target) {
  mount(JarvisApp, { target });
}

export {};
