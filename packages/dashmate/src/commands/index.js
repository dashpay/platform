// Iterate over all files in this directory and build an explicit list of commands
//
// import { glob } from 'glob';
// import path from 'path';
// import { fileURLToPath } from 'url';
//
// const currentFile = fileURLToPath(import.meta.url);
// const currentDir = path.dirname(currentFile);

// eslint-disable-next-line import/prefer-default-export
export const COMMANDS = {};
//
// for (const file of glob.sync(`${currentDir}/**/*.js`)) {
//   if (file === currentFile) {
//     continue;
//   }
//
//   const relativePath = path.relative(currentDir, file);
//   const dirName = path.dirname(relativePath);
//   let baseName = path.basename(relativePath, '.js');
//
//   // If the baseName is 'index', it means the command is the directory itself
//   if (baseName === 'index') {
//     baseName = '';
//   }
//
//   const commandName = path.join(dirName, baseName).replace(path.sep, ' ').trim();
//
//   COMMANDS[commandName] = (await import(`./${relativePath}`)).default;
// }

COMMANDS['setup'] = (await import('./setup.js')).default;

console.dir(COMMANDS);
