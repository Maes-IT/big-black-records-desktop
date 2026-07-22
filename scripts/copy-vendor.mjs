// Copie les builds UMD (prêts pour <script>, sans bundler) des libs d'export
// depuis node_modules vers src/vendor, pour qu'ils soient servis par Tauri
// (frontendDist = ../src, donc rien en dehors de ce dossier n'est accessible).
import { copyFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const destDir = join(root, 'src', 'vendor');
mkdirSync(destDir, { recursive: true });

const files = [
  ['node_modules/jspdf/dist/jspdf.umd.min.js', 'jspdf.umd.min.js'],
  ['node_modules/jspdf-autotable/dist/jspdf.plugin.autotable.min.js', 'jspdf.plugin.autotable.min.js'],
  ['node_modules/xlsx/dist/xlsx.full.min.js', 'xlsx.full.min.js'],
];

for (const [src, destName] of files) {
  copyFileSync(join(root, src), join(destDir, destName));
  console.log(`vendored: ${destName}`);
}
