#!/usr/bin/env node

/**
 * Generate release notes for WASM SDK
 * Supports conventional commits and automated changelog generation
 */

import { execSync } from 'child_process';
import { writeFileSync, readFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const PACKAGE_ROOT = join(__dirname, '..');
const PKG_PATH = join(PACKAGE_ROOT, 'pkg', 'package.json');

/**
 * Get package version from pkg/package.json
 */
function getPackageVersion() {
  if (!existsSync(PKG_PATH)) {
    throw new Error('Package not found. Run build first.');
  }
  const pkg = JSON.parse(readFileSync(PKG_PATH, 'utf8'));
  return pkg.version;
}

/**
 * Get git commits since last release
 */
function getCommitsSinceLastRelease(lastTag) {
  try {
    let gitCmd;
    if (lastTag) {
      gitCmd = `git log ${lastTag}..HEAD --pretty=format:"%h|%s|%an|%ad" --date=short --grep="wasm-sdk" --grep="Issue #" --all-match`;
    } else {
      gitCmd = 'git log --pretty=format:"%h|%s|%an|%ad" --date=short --grep="wasm-sdk" --grep="Issue #" --all-match -n 20';
    }
    
    const output = execSync(gitCmd, { cwd: PACKAGE_ROOT, encoding: 'utf8' });
    return output.split('\n').filter(line => line.trim()).map(line => {
      const [hash, message, author, date] = line.split('|');
      return { hash, message, author, date };
    });
  } catch (error) {
    console.warn('Warning: Could not get git commits:', error.message);
    return [];
  }
}

/**
 * Get last release tag
 */
function getLastReleaseTag() {
  try {
    const tags = execSync('git tag -l "wasm-sdk-v*" --sort=-version:refname', { 
      cwd: PACKAGE_ROOT, 
      encoding: 'utf8' 
    });
    return tags.split('\n')[0] || null;
  } catch (error) {
    return null;
  }
}

/**
 * Categorize commits by type
 */
function categorizeCommits(commits) {
  const categories = {
    breaking: [],
    features: [],
    fixes: [],
    performance: [],
    documentation: [],
    tests: [],
    refactor: [],
    build: [],
    other: []
  };

  commits.forEach(commit => {
    const msg = commit.message.toLowerCase();
    
    if (msg.includes('breaking') || msg.includes('!:')) {
      categories.breaking.push(commit);
    } else if (msg.startsWith('feat') || msg.includes('add:') || msg.includes('new:')) {
      categories.features.push(commit);
    } else if (msg.startsWith('fix') || msg.includes('bug') || msg.includes('issue')) {
      categories.fixes.push(commit);
    } else if (msg.includes('perf') || msg.includes('performance') || msg.includes('optimize')) {
      categories.performance.push(commit);
    } else if (msg.includes('docs') || msg.includes('documentation') || msg.includes('readme')) {
      categories.documentation.push(commit);
    } else if (msg.includes('test') || msg.includes('spec')) {
      categories.tests.push(commit);
    } else if (msg.includes('refactor') || msg.includes('cleanup') || msg.includes('reorganize')) {
      categories.refactor.push(commit);
    } else if (msg.includes('build') || msg.includes('deps') || msg.includes('dependency')) {
      categories.build.push(commit);
    } else {
      categories.other.push(commit);
    }
  });

  return categories;
}

/**
 * Generate markdown release notes
 */
function generateReleaseNotes(version, commits, lastTag) {
  const categories = categorizeCommits(commits);
  const date = new Date().toISOString().split('T')[0];
  
  let notes = `# WASM SDK v${version} Release Notes\n\n`;
  notes += `**Release Date:** ${date}\n\n`;
  
  if (lastTag) {
    notes += `**Changes since:** ${lastTag}\n\n`;
  }
  
  // Installation section
  notes += '## Installation\n\n';
  notes += '```bash\n';
  notes += `npm install @dashevo/dash-wasm-sdk@${version}\n`;
  notes += '```\n\n';
  
  // Breaking changes (highest priority)
  if (categories.breaking.length > 0) {
    notes += '## ⚠️ Breaking Changes\n\n';
    categories.breaking.forEach(commit => {
      notes += `- **${commit.hash}**: ${commit.message} (@${commit.author})\n`;
    });
    notes += '\n';
  }
  
  // New features
  if (categories.features.length > 0) {
    notes += '## ✨ New Features\n\n';
    categories.features.forEach(commit => {
      notes += `- **${commit.hash}**: ${commit.message} (@${commit.author})\n`;
    });
    notes += '\n';
  }
  
  // Bug fixes
  if (categories.fixes.length > 0) {
    notes += '## 🐛 Bug Fixes\n\n';
    categories.fixes.forEach(commit => {
      notes += `- **${commit.hash}**: ${commit.message} (@${commit.author})\n`;
    });
    notes += '\n';
  }
  
  // Performance improvements
  if (categories.performance.length > 0) {
    notes += '## 🚀 Performance Improvements\n\n';
    categories.performance.forEach(commit => {
      notes += `- **${commit.hash}**: ${commit.message} (@${commit.author})\n`;
    });
    notes += '\n';
  }
  
  // Documentation updates
  if (categories.documentation.length > 0) {
    notes += '## 📚 Documentation\n\n';
    categories.documentation.forEach(commit => {
      notes += `- **${commit.hash}**: ${commit.message} (@${commit.author})\n`;
    });
    notes += '\n';
  }
  
  // Tests and refactoring (lower priority, collapsed by default)
  if (categories.tests.length > 0 || categories.refactor.length > 0 || categories.build.length > 0) {
    notes += '<details>\n';
    notes += '<summary>🔧 Internal Changes</summary>\n\n';
    
    if (categories.tests.length > 0) {
      notes += '### Tests\n';
      categories.tests.forEach(commit => {
        notes += `- **${commit.hash}**: ${commit.message} (@${commit.author})\n`;
      });
      notes += '\n';
    }
    
    if (categories.refactor.length > 0) {
      notes += '### Refactoring\n';
      categories.refactor.forEach(commit => {
        notes += `- **${commit.hash}**: ${commit.message} (@${commit.author})\n`;
      });
      notes += '\n';
    }
    
    if (categories.build.length > 0) {
      notes += '### Build & Dependencies\n';
      categories.build.forEach(commit => {
        notes += `- **${commit.hash}**: ${commit.message} (@${commit.author})\n`;
      });
      notes += '\n';
    }
    
    notes += '</details>\n\n';
  }
  
  // Other changes
  if (categories.other.length > 0) {
    notes += '<details>\n';
    notes += '<summary>📋 Other Changes</summary>\n\n';
    categories.other.forEach(commit => {
      notes += `- **${commit.hash}**: ${commit.message} (@${commit.author})\n`;
    });
    notes += '\n</details>\n\n';
  }
  
  // Links and support
  notes += '## 📖 Resources\n\n';
  notes += '- [API Documentation](https://dashplatform.readme.io/)\n';
  notes += '- [GitHub Repository](https://github.com/dashpay/platform)\n';
  notes += '- [Issue Tracker](https://github.com/dashpay/platform/issues)\n';
  notes += '- [Community Support](https://github.com/dashpay/platform/discussions)\n\n';
  
  notes += '## 🚨 Support\n\n';
  notes += 'If you encounter any issues or have questions:\n';
  notes += '1. Check the [documentation](https://dashplatform.readme.io/)\n';
  notes += '2. Search [existing issues](https://github.com/dashpay/platform/issues)\n';
  notes += '3. Create a [new issue](https://github.com/dashpay/platform/issues/new/choose)\n';
  notes += '4. Join the [community discussion](https://github.com/dashpay/platform/discussions)\n\n';
  
  return notes;
}

/**
 * Update CHANGELOG.md
 */
function updateChangelog(releaseNotes) {
  const changelogPath = join(PACKAGE_ROOT, 'CHANGELOG.md');
  let changelog = '';
  
  if (existsSync(changelogPath)) {
    changelog = readFileSync(changelogPath, 'utf8');
  } else {
    changelog = '# Changelog\n\nAll notable changes to the WASM SDK will be documented in this file.\n\n';
  }
  
  // Insert new release notes after the header
  const headerEnd = changelog.indexOf('\n\n') + 2;
  const newChangelog = changelog.substring(0, headerEnd) + releaseNotes + '\n---\n\n' + changelog.substring(headerEnd);
  
  writeFileSync(changelogPath, newChangelog);
  console.log(`✅ Updated ${changelogPath}`);
}

/**
 * Main execution
 */
function main() {
  try {
    console.log('🔄 Generating WASM SDK release notes...');
    
    const version = getPackageVersion();
    console.log(`📦 Version: ${version}`);
    
    const lastTag = getLastReleaseTag();
    console.log(`🏷️  Last tag: ${lastTag || 'none'}`);
    
    const commits = getCommitsSinceLastRelease(lastTag);
    console.log(`📝 Found ${commits.length} commits`);
    
    const releaseNotes = generateReleaseNotes(version, commits, lastTag);
    
    // Write release notes to file
    const notesPath = join(PACKAGE_ROOT, `RELEASE_NOTES_v${version}.md`);
    writeFileSync(notesPath, releaseNotes);
    console.log(`✅ Generated ${notesPath}`);
    
    // Update changelog
    updateChangelog(releaseNotes);
    
    // Output to stdout for GitHub Actions
    if (process.env.CI) {
      console.log('\n--- RELEASE NOTES ---\n');
      console.log(releaseNotes);
      console.log('\n--- END RELEASE NOTES ---\n');
    }
    
    console.log('✅ Release notes generation completed successfully');
    
  } catch (error) {
    console.error('❌ Error generating release notes:', error.message);
    process.exit(1);
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}