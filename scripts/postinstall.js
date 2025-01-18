import { readFileSync, writeFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const xhrProxyPath = join(__dirname, '..', 'node_modules', 'xhr-fetch-proxy', 'index.js');
const targetPath = join(__dirname, '..', 'index.js');

const xhrProxyContent = readFileSync(xhrProxyPath, 'utf8');
const mainContent = readFileSync(targetPath, 'utf8');

const startMarker = '// BEGIN XHR-FETCH-PROXY\n';
const endMarker = '// END XHR-FETCH-PROXY';

const regex = new RegExp(`${startMarker}[\\s\\S]*${endMarker}`);
const cleanedContent = mainContent.replace(regex, '').trim();

const newContent = `${cleanedContent}\n\n${startMarker}${xhrProxyContent}\n${endMarker}`;

writeFileSync(targetPath, newContent);