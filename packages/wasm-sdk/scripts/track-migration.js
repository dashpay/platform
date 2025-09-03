#!/usr/bin/env node

/**
 * Migration Progress Tracking Script
 * Automatically updates migration status and generates progress reports
 */

import { readFileSync, writeFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { execSync } from 'child_process';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const PACKAGE_ROOT = join(__dirname, '..');
const TRACKING_FILE = join(PACKAGE_ROOT, 'MIGRATION_TRACKING.md');
const PROGRESS_FILE = join(PACKAGE_ROOT, 'migration-progress.json');

/**
 * Migration feature definitions
 */
const MIGRATION_FEATURES = {
  'core-client': {
    name: 'DashPlatformClient',
    jsSDKApi: 'new DashPlatformSDK()',
    priority: 'Critical',
    complexity: 'High',
    status: 'planned',
    milestone: 1,
    eta: '2024-Q2'
  },
  'network-config': {
    name: 'Network Configuration',
    jsSDKApi: 'sdk.getNetwork()',
    priority: 'Critical', 
    complexity: 'Medium',
    status: 'planned',
    milestone: 1,
    eta: '2024-Q2'
  },
  'connection-mgmt': {
    name: 'Connection Management',
    jsSDKApi: 'sdk.connect()',
    priority: 'Critical',
    complexity: 'Medium',
    status: 'planned',
    milestone: 1,
    eta: '2024-Q2'
  },
  'error-handling': {
    name: 'Error Handling',
    jsSDKApi: 'DashSDKError',
    priority: 'Critical',
    complexity: 'Low',
    status: 'planned',
    milestone: 1,
    eta: '2024-Q2'
  },
  'identity-creation': {
    name: 'Identity Creation',
    jsSDKApi: 'platform.identities.register()',
    priority: 'Critical',
    complexity: 'High',
    status: 'planned',
    milestone: 2,
    eta: '2024-Q2'
  },
  'identity-retrieval': {
    name: 'Identity Retrieval',
    jsSDKApi: 'platform.identities.get()',
    priority: 'Critical',
    complexity: 'Medium',
    status: 'planned',
    milestone: 2,
    eta: '2024-Q2'
  },
  'document-creation': {
    name: 'Document Creation',
    jsSDKApi: 'platform.documents.create()',
    priority: 'Critical',
    complexity: 'Medium',
    status: 'planned',
    milestone: 2,
    eta: '2024-Q2'
  },
  'document-query': {
    name: 'Document Query',
    jsSDKApi: 'platform.documents.query()',
    priority: 'Critical',
    complexity: 'High',
    status: 'planned',
    milestone: 3,
    eta: '2024-Q3'
  },
  'contract-creation': {
    name: 'Contract Creation',
    jsSDKApi: 'platform.contracts.create()',
    priority: 'Critical',
    complexity: 'High',
    status: 'planned',
    milestone: 3,
    eta: '2024-Q3'
  },
  'wallet-integration': {
    name: 'Wallet Integration',
    jsSDKApi: 'new Wallet()',
    priority: 'High',
    complexity: 'High',
    status: 'planned',
    milestone: 4,
    eta: '2024-Q3'
  }
  // Add more features as needed
};

/**
 * Milestone definitions
 */
const MILESTONES = {
  1: {
    name: 'Core Client Infrastructure',
    target: '2024-Q2',
    features: ['core-client', 'network-config', 'connection-mgmt', 'error-handling']
  },
  2: {
    name: 'Essential Operations', 
    target: '2024-Q2',
    features: ['identity-creation', 'identity-retrieval', 'document-creation']
  },
  3: {
    name: 'Advanced Document Operations',
    target: '2024-Q3',
    features: ['document-query', 'contract-creation']
  },
  4: {
    name: 'Wallet & Security',
    target: '2024-Q3',
    features: ['wallet-integration']
  }
};

/**
 * Load current progress from JSON file
 */
function loadProgress() {
  if (existsSync(PROGRESS_FILE)) {
    return JSON.parse(readFileSync(PROGRESS_FILE, 'utf8'));
  }
  
  // Initialize progress tracking
  const progress = {
    lastUpdated: new Date().toISOString(),
    features: {},
    milestones: {},
    overall: {
      total: Object.keys(MIGRATION_FEATURES).length,
      completed: 0,
      inProgress: 0,
      planned: Object.keys(MIGRATION_FEATURES).length
    }
  };
  
  // Initialize feature status
  for (const [featureId, feature] of Object.entries(MIGRATION_FEATURES)) {
    progress.features[featureId] = {
      ...feature,
      startDate: null,
      completedDate: null,
      assignee: null,
      notes: ''
    };
  }
  
  // Initialize milestone status
  for (const [milestoneId, milestone] of Object.entries(MILESTONES)) {
    progress.milestones[milestoneId] = {
      ...milestone,
      status: 'planned',
      progress: 0,
      completedFeatures: 0,
      totalFeatures: milestone.features.length
    };
  }
  
  return progress;
}

/**
 * Save progress to JSON file
 */
function saveProgress(progress) {
  progress.lastUpdated = new Date().toISOString();
  writeFileSync(PROGRESS_FILE, JSON.stringify(progress, null, 2));
}

/**
 * Update feature status
 */
function updateFeature(featureId, updates) {
  const progress = loadProgress();
  
  if (!progress.features[featureId]) {
    throw new Error(`Feature ${featureId} not found`);
  }
  
  const oldStatus = progress.features[featureId].status;
  const newStatus = updates.status || oldStatus;
  
  // Update feature
  progress.features[featureId] = {
    ...progress.features[featureId],
    ...updates
  };
  
  // Update timestamps
  if (newStatus === 'in_progress' && oldStatus !== 'in_progress') {
    progress.features[featureId].startDate = new Date().toISOString();
  }
  
  if (newStatus === 'completed' && oldStatus !== 'completed') {
    progress.features[featureId].completedDate = new Date().toISOString();
  }
  
  // Recalculate overall stats
  calculateOverallProgress(progress);
  
  // Update milestone progress
  updateMilestoneProgress(progress);
  
  saveProgress(progress);
  
  console.log(`✅ Updated ${featureId}: ${oldStatus} → ${newStatus}`);
}

/**
 * Calculate overall progress statistics
 */
function calculateOverallProgress(progress) {
  let completed = 0;
  let inProgress = 0;
  let planned = 0;
  
  for (const feature of Object.values(progress.features)) {
    switch (feature.status) {
      case 'completed':
        completed++;
        break;
      case 'in_progress':
        inProgress++;
        break;
      case 'planned':
        planned++;
        break;
    }
  }
  
  progress.overall = {
    total: Object.keys(progress.features).length,
    completed,
    inProgress,
    planned
  };
}

/**
 * Update milestone progress based on feature status
 */
function updateMilestoneProgress(progress) {
  for (const [milestoneId, milestone] of Object.entries(MILESTONES)) {
    let completedFeatures = 0;
    
    for (const featureId of milestone.features) {
      if (progress.features[featureId]?.status === 'completed') {
        completedFeatures++;
      }
    }
    
    const progressPercent = (completedFeatures / milestone.features.length) * 100;
    
    progress.milestones[milestoneId] = {
      ...progress.milestones[milestoneId],
      completedFeatures,
      progress: Math.round(progressPercent)
    };
    
    // Update milestone status
    if (progressPercent === 100) {
      progress.milestones[milestoneId].status = 'completed';
    } else if (progressPercent > 0) {
      progress.milestones[milestoneId].status = 'in_progress';
    }
  }
}

/**
 * Generate progress report
 */
function generateReport() {
  const progress = loadProgress();
  const now = new Date().toISOString().split('T')[0];
  
  let report = `# WASM SDK Phase 2 Migration Progress Report\n\n`;
  report += `**Generated**: ${now}\n`;
  report += `**Last Updated**: ${progress.lastUpdated.split('T')[0]}\n\n`;
  
  // Overall progress
  const overall = progress.overall;
  const overallPercent = Math.round((overall.completed / overall.total) * 100);
  
  report += `## 📊 Overall Progress\n\n`;
  report += `- **Total Features**: ${overall.total}\n`;
  report += `- **Completed**: ${overall.completed} (${overallPercent}%)\n`;
  report += `- **In Progress**: ${overall.inProgress}\n`;
  report += `- **Planned**: ${overall.planned}\n\n`;
  
  // Progress bar
  const barLength = 30;
  const completedBar = Math.round((overall.completed / overall.total) * barLength);
  const inProgressBar = Math.round((overall.inProgress / overall.total) * barLength);
  const progressBar = '█'.repeat(completedBar) + 
                     '▓'.repeat(inProgressBar) + 
                     '░'.repeat(barLength - completedBar - inProgressBar);
  
  report += `\`\`\`\n${progressBar} ${overallPercent}%\n\`\`\`\n\n`;
  
  // Milestone progress
  report += `## 🎯 Milestone Progress\n\n`;
  for (const [milestoneId, milestone] of Object.entries(progress.milestones)) {
    const statusEmoji = milestone.status === 'completed' ? '✅' :
                       milestone.status === 'in_progress' ? '🔄' : '📋';
    
    report += `### ${statusEmoji} Milestone ${milestoneId}: ${milestone.name}\n`;
    report += `- **Progress**: ${milestone.progress}% (${milestone.completedFeatures}/${milestone.totalFeatures})\n`;
    report += `- **Target**: ${milestone.target}\n`;
    report += `- **Status**: ${milestone.status}\n\n`;
  }
  
  // Recent activity
  report += `## 📝 Recent Activity\n\n`;
  const recentFeatures = Object.entries(progress.features)
    .filter(([_, feature]) => feature.completedDate || feature.startDate)
    .sort((a, b) => {
      const dateA = a[1].completedDate || a[1].startDate;
      const dateB = b[1].completedDate || b[1].startDate;
      return new Date(dateB) - new Date(dateA);
    })
    .slice(0, 5);
  
  if (recentFeatures.length > 0) {
    for (const [featureId, feature] of recentFeatures) {
      const date = (feature.completedDate || feature.startDate).split('T')[0];
      const action = feature.completedDate ? 'Completed' : 'Started';
      report += `- **${date}**: ${action} ${feature.name}\n`;
    }
  } else {
    report += `No recent activity to report.\n`;
  }
  
  report += `\n## 📋 Next Actions\n\n`;
  
  // Features ready to start
  const plannedFeatures = Object.entries(progress.features)
    .filter(([_, feature]) => feature.status === 'planned')
    .slice(0, 3);
    
  if (plannedFeatures.length > 0) {
    report += `### Ready to Start\n`;
    for (const [featureId, feature] of plannedFeatures) {
      report += `- **${feature.name}** (${feature.priority} priority, ${feature.complexity} complexity)\n`;
    }
  }
  
  report += `\n---\n\n`;
  report += `*Report generated automatically by migration tracking system*\n`;
  
  return report;
}

/**
 * Update tracking documentation
 */
function updateDocumentation() {
  const progress = loadProgress();
  
  // Read current tracking document
  let content = readFileSync(TRACKING_FILE, 'utf8');
  
  // Update overall statistics section
  const overall = progress.overall;
  const overallPercent = Math.round((overall.completed / overall.total) * 100);
  
  const statsSection = `### Overall Statistics
- **Total Features to Migrate**: ${overall.total}
- **Completed Features**: ${overall.completed} (${overallPercent}%)
- **In Progress Features**: ${overall.inProgress} (${Math.round((overall.inProgress / overall.total) * 100)}%)
- **Planned Features**: ${overall.planned} (${Math.round((overall.planned / overall.total) * 100)}%)`;
  
  // Replace statistics section
  content = content.replace(
    /### Overall Statistics[\s\S]*?(?=### Phase Status)/,
    statsSection + '\n\n'
  );
  
  writeFileSync(TRACKING_FILE, content);
  console.log('📄 Updated MIGRATION_TRACKING.md');
}

/**
 * Main CLI interface
 */
function main() {
  const args = process.argv.slice(2);
  const command = args[0];
  
  try {
    switch (command) {
      case 'init':
        const progress = loadProgress();
        saveProgress(progress);
        console.log('✅ Initialized migration tracking');
        break;
        
      case 'update':
        const featureId = args[1];
        const status = args[2];
        const assignee = args[3];
        
        if (!featureId || !status) {
          console.error('Usage: update <feature-id> <status> [assignee]');
          process.exit(1);
        }
        
        updateFeature(featureId, { 
          status, 
          assignee: assignee || null 
        });
        break;
        
      case 'report':
        const report = generateReport();
        console.log(report);
        
        // Save report to file
        const reportFile = join(PACKAGE_ROOT, 'MIGRATION_REPORT.md');
        writeFileSync(reportFile, report);
        console.log(`📄 Report saved to ${reportFile}`);
        break;
        
      case 'status':
        const currentProgress = loadProgress();
        console.log('📊 Current Migration Status:');
        console.log(`   Total: ${currentProgress.overall.total}`);
        console.log(`   Completed: ${currentProgress.overall.completed}`);
        console.log(`   In Progress: ${currentProgress.overall.inProgress}`);
        console.log(`   Planned: ${currentProgress.overall.planned}`);
        break;
        
      case 'list':
        const listProgress = loadProgress();
        console.log('📋 All Features:');
        for (const [featureId, feature] of Object.entries(listProgress.features)) {
          const statusEmoji = feature.status === 'completed' ? '✅' :
                             feature.status === 'in_progress' ? '🔄' : '📋';
          console.log(`   ${statusEmoji} ${featureId}: ${feature.name} (${feature.status})`);
        }
        break;
        
      case 'docs':
        updateDocumentation();
        break;
        
      default:
        console.log('WASM SDK Migration Tracking Tool');
        console.log('');
        console.log('Commands:');
        console.log('  init                     Initialize tracking system');
        console.log('  update <id> <status>     Update feature status');
        console.log('  report                   Generate progress report');
        console.log('  status                   Show current status');
        console.log('  list                     List all features');
        console.log('  docs                     Update documentation');
        console.log('');
        console.log('Status values: planned, in_progress, completed, blocked, cancelled');
        console.log('');
        console.log('Examples:');
        console.log('  ./track-migration.js update core-client in_progress');
        console.log('  ./track-migration.js update identity-creation completed');
        break;
    }
  } catch (error) {
    console.error('❌ Error:', error.message);
    process.exit(1);
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}