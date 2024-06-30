const fs = require('fs');
const path = require('path');

// Step 1: Read the build.js file and extract the external array
const buildJsContent = fs.readFileSync(path.join(__dirname, '../packages/dashmate/scripts/build.js'), 'utf8');
const externalMatch = buildJsContent.match(/external: \[([^\]]+)\]/);
if (!externalMatch) {
  console.error('Could not find external array in build.js');
  process.exit(1);
}
const external = externalMatch[1].split(',').map(s => s.trim().replace(/['"]/g, ''));

// Step 2: Read the package.json file and parse it into a JSON object
const packageJsonPath = path.join(__dirname, '../packages/dashmate/package/package.json');
const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));

// Step 3: Iterate over the dependencies in the package.json object and remove any that are not in the external array
for (const dep in packageJson.dependencies) {
  if (!external.includes(dep)) {
    delete packageJson.dependencies[dep];
  }
}

// Step 3: Iterate over the dependencies in the package.json object and remove any that are not in the external array
for (const dep in packageJson.devDependencies) {
  if (dep !== 'oclif') {
    delete packageJson.devDependencies[dep];
  }
}

// Step 5: Write the modified package.json object back to the package.json file
fs.writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2));
