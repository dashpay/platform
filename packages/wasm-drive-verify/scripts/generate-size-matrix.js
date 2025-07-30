#!/usr/bin/env node

import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(__dirname, '..');

// Configuration
const FEATURES = ['identity', 'document', 'contract', 'tokens', 'governance', 'transitions'];
const BASE_FEATURES = 'console_error_panic_hook';

// Helper functions
function formatBytes(bytes) {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function getFileSize(filePath) {
  try {
    return fs.statSync(filePath).size;
  } catch (e) {
    return 0;
  }
}

// Generate all possible combinations
function generateCombinations(features) {
  const combinations = [];
  const n = features.length;
  
  // Add empty combination (base only)
  combinations.push({
    name: 'base',
    features: [],
    featureString: BASE_FEATURES
  });
  
  // Generate all possible combinations
  for (let i = 1; i < Math.pow(2, n); i++) {
    const combo = [];
    let name = '';
    
    for (let j = 0; j < n; j++) {
      if (i & (1 << j)) {
        combo.push(features[j]);
        name += features[j].substring(0, 3);
      }
    }
    
    combinations.push({
      name: combo.length === 1 ? combo[0] : name,
      features: combo,
      featureString: [BASE_FEATURES, ...combo].join(',')
    });
  }
  
  // Add full combination
  combinations.push({
    name: 'full',
    features: ['full'],
    featureString: BASE_FEATURES + ',full'
  });
  
  return combinations;
}

// Build a specific combination
function buildCombination(combination, outputDir) {
  console.log(`Building ${combination.name}...`);
  
  const buildDir = path.join(outputDir, combination.name);
  fs.mkdirSync(buildDir, { recursive: true });
  
  try {
    // Build with cargo
    execSync(
      `cargo build --target wasm32-unknown-unknown --release --no-default-features --features "${combination.featureString}"`,
      { cwd: projectRoot, stdio: 'pipe' }
    );
    
    // Run wasm-bindgen
    execSync(
      `wasm-bindgen ${path.join(projectRoot, '../../target/wasm32-unknown-unknown/release/wasm_drive_verify.wasm')} --out-dir ${buildDir} --target web --out-name bundle`,
      { stdio: 'pipe' }
    );
    
    // Get sizes
    const wasmSize = getFileSize(path.join(buildDir, 'bundle_bg.wasm'));
    const jsSize = getFileSize(path.join(buildDir, 'bundle.js'));
    
    // Try to optimize with wasm-opt
    let optimizedSize = wasmSize;
    try {
      execSync(
        `wasm-opt -Oz ${path.join(buildDir, 'bundle_bg.wasm')} -o ${path.join(buildDir, 'bundle_bg_opt.wasm')}`,
        { stdio: 'pipe' }
      );
      optimizedSize = getFileSize(path.join(buildDir, 'bundle_bg_opt.wasm'));
    } catch (e) {
      // wasm-opt not available
    }
    
    return {
      ...combination,
      wasmSize,
      jsSize,
      optimizedSize,
      totalSize: wasmSize + jsSize
    };
  } catch (error) {
    console.error(`Failed to build ${combination.name}: ${error.message}`);
    return {
      ...combination,
      wasmSize: 0,
      jsSize: 0,
      optimizedSize: 0,
      totalSize: 0,
      error: error.message
    };
  }
}

// Generate size matrix
function generateMatrix(results) {
  const matrix = {};
  
  // Initialize matrix
  FEATURES.forEach(f1 => {
    matrix[f1] = {};
    FEATURES.forEach(f2 => {
      matrix[f1][f2] = null;
    });
  });
  
  // Fill matrix with combination sizes
  results.forEach(result => {
    if (result.features.length === 2) {
      const [f1, f2] = result.features;
      matrix[f1][f2] = result.wasmSize;
      matrix[f2][f1] = result.wasmSize;
    }
  });
  
  return matrix;
}

// Generate HTML visualization
function generateVisualization(results, outputPath) {
  const baseSize = results.find(r => r.name === 'base').wasmSize;
  const fullSize = results.find(r => r.name === 'full').wasmSize;
  
  const html = `
<!DOCTYPE html>
<html>
<head>
    <title>WASM Drive Verify - Module Size Analysis</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 20px;
            background: #f5f5f5;
        }
        .container {
            max-width: 1200px;
            margin: 0 auto;
            background: white;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        h1, h2 {
            color: #333;
        }
        .stats {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 15px;
            margin: 20px 0;
        }
        .stat-card {
            background: #f8f9fa;
            padding: 15px;
            border-radius: 4px;
            border-left: 4px solid #007bff;
        }
        .stat-value {
            font-size: 24px;
            font-weight: bold;
            color: #007bff;
        }
        .stat-label {
            color: #666;
            font-size: 14px;
        }
        table {
            width: 100%;
            border-collapse: collapse;
            margin: 20px 0;
        }
        th, td {
            padding: 10px;
            text-align: left;
            border-bottom: 1px solid #e0e0e0;
        }
        th {
            background: #f8f9fa;
            font-weight: 600;
        }
        tr:hover {
            background: #f8f9fa;
        }
        .size-bar {
            display: inline-block;
            height: 20px;
            background: linear-gradient(90deg, #4CAF50 0%, #FFC107 50%, #F44336 100%);
            border-radius: 2px;
            margin-right: 8px;
        }
        .reduction {
            color: #4CAF50;
            font-weight: 600;
        }
        .chart-container {
            margin: 30px 0;
        }
        .combination-grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
            gap: 10px;
            margin: 20px 0;
        }
        .combo-card {
            background: #f8f9fa;
            padding: 10px;
            border-radius: 4px;
            text-align: center;
            transition: all 0.2s;
        }
        .combo-card:hover {
            transform: translateY(-2px);
            box-shadow: 0 4px 8px rgba(0,0,0,0.1);
        }
        .combo-size {
            font-size: 18px;
            font-weight: bold;
            color: #007bff;
        }
        .combo-name {
            font-size: 12px;
            color: #666;
            margin-top: 4px;
        }
    </style>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
</head>
<body>
    <div class="container">
        <h1>WASM Drive Verify - Module Size Analysis</h1>
        
        <div class="stats">
            <div class="stat-card">
                <div class="stat-value">${formatBytes(baseSize)}</div>
                <div class="stat-label">Base Size</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">${formatBytes(fullSize)}</div>
                <div class="stat-label">Full Bundle</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">${((1 - baseSize / fullSize) * 100).toFixed(1)}%</div>
                <div class="stat-label">Max Reduction</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">${results.length}</div>
                <div class="stat-label">Combinations Tested</div>
            </div>
        </div>

        <h2>Module Combinations</h2>
        <table>
            <thead>
                <tr>
                    <th>Combination</th>
                    <th>Features</th>
                    <th>WASM Size</th>
                    <th>JS Size</th>
                    <th>Total Size</th>
                    <th>Reduction</th>
                    <th>Visual</th>
                </tr>
            </thead>
            <tbody>
                ${results
                  .sort((a, b) => a.totalSize - b.totalSize)
                  .map(r => {
                    const reduction = ((1 - r.wasmSize / fullSize) * 100).toFixed(1);
                    const barWidth = (r.wasmSize / fullSize * 100).toFixed(0);
                    return `
                    <tr>
                        <td><strong>${r.name}</strong></td>
                        <td>${r.features.join(', ') || 'core only'}</td>
                        <td>${formatBytes(r.wasmSize)}</td>
                        <td>${formatBytes(r.jsSize)}</td>
                        <td>${formatBytes(r.totalSize)}</td>
                        <td class="reduction">${r.name === 'full' ? 'baseline' : reduction + '%'}</td>
                        <td><div class="size-bar" style="width: ${barWidth}%"></div></td>
                    </tr>
                    `;
                  }).join('')}
            </tbody>
        </table>

        <h2>Size by Feature Count</h2>
        <div class="chart-container">
            <canvas id="featureCountChart"></canvas>
        </div>

        <h2>Individual Module Sizes</h2>
        <div class="combination-grid">
            ${FEATURES.map(feature => {
              const result = results.find(r => r.name === feature);
              const moduleSize = result ? result.wasmSize - baseSize : 0;
              return `
                <div class="combo-card">
                    <div class="combo-size">${formatBytes(moduleSize)}</div>
                    <div class="combo-name">${feature}</div>
                </div>
              `;
            }).join('')}
        </div>

        <h2>Common Use Cases</h2>
        <div class="combination-grid">
            ${[
              { name: 'Identity App', features: ['identity'] },
              { name: 'DeFi App', features: ['identity', 'tokens', 'contract'] },
              { name: 'Document Storage', features: ['document', 'contract'] },
              { name: 'Governance App', features: ['governance'] },
              { name: 'Lite Client', features: ['identity', 'document'] }
            ].map(useCase => {
              const result = results.find(r => 
                r.features.length === useCase.features.length &&
                useCase.features.every(f => r.features.includes(f))
              );
              return result ? `
                <div class="combo-card">
                    <div class="combo-size">${formatBytes(result.wasmSize)}</div>
                    <div class="combo-name">${useCase.name}</div>
                </div>
              ` : '';
            }).join('')}
        </div>
    </div>

    <script>
        // Feature count chart
        const featureCounts = {};
        ${JSON.stringify(results)}.forEach(r => {
            const count = r.features.filter(f => f !== 'full').length;
            if (!featureCounts[count]) featureCounts[count] = [];
            featureCounts[count].push(r.wasmSize);
        });

        const avgSizes = Object.entries(featureCounts).map(([count, sizes]) => ({
            x: parseInt(count),
            y: sizes.reduce((a, b) => a + b, 0) / sizes.length / 1024 // KB
        })).sort((a, b) => a.x - b.x);

        new Chart(document.getElementById('featureCountChart'), {
            type: 'line',
            data: {
                datasets: [{
                    label: 'Average Size (KB)',
                    data: avgSizes,
                    borderColor: '#007bff',
                    backgroundColor: 'rgba(0, 123, 255, 0.1)',
                    tension: 0.4
                }]
            },
            options: {
                responsive: true,
                plugins: {
                    title: {
                        display: true,
                        text: 'Average Bundle Size by Number of Features'
                    }
                },
                scales: {
                    x: {
                        type: 'linear',
                        title: {
                            display: true,
                            text: 'Number of Features'
                        },
                        ticks: {
                            stepSize: 1
                        }
                    },
                    y: {
                        title: {
                            display: true,
                            text: 'Size (KB)'
                        }
                    }
                }
            }
        });
    </script>
</body>
</html>
  `;
  
  fs.writeFileSync(outputPath, html);
}

// Main execution
async function main() {
  console.log('=== WASM Drive Verify Size Matrix Analysis ===\n');
  
  const outputDir = path.join(projectRoot, 'size-analysis');
  fs.rmSync(outputDir, { recursive: true, force: true });
  fs.mkdirSync(outputDir, { recursive: true });
  
  // Generate all combinations
  const combinations = generateCombinations(FEATURES);
  console.log(`Generated ${combinations.length} combinations to test\n`);
  
  // Build each combination
  const results = [];
  for (const combo of combinations) {
    const result = buildCombination(combo, outputDir);
    results.push(result);
    
    // Show progress
    process.stdout.write(`\rProgress: ${results.length}/${combinations.length} combinations built`);
  }
  console.log('\n');
  
  // Generate reports
  const reportPath = path.join(outputDir, 'report.json');
  fs.writeFileSync(reportPath, JSON.stringify(results, null, 2));
  
  const htmlPath = path.join(outputDir, 'visualization.html');
  generateVisualization(results, htmlPath);
  
  // Generate CSV for further analysis
  const csvPath = path.join(outputDir, 'results.csv');
  const csv = [
    'name,features,wasm_size,js_size,optimized_size,total_size,reduction',
    ...results.map(r => {
      const fullSize = results.find(res => res.name === 'full').wasmSize;
      const reduction = r.name === 'full' ? 0 : ((1 - r.wasmSize / fullSize) * 100).toFixed(1);
      return `${r.name},"${r.features.join(',')}",${r.wasmSize},${r.jsSize},${r.optimizedSize},${r.totalSize},${reduction}`;
    })
  ].join('\n');
  fs.writeFileSync(csvPath, csv);
  
  // Print summary
  console.log('=== Analysis Complete ===\n');
  console.log(`Results saved to: ${outputDir}/`);
  console.log(`- report.json: Complete results data`);
  console.log(`- results.csv: CSV format for spreadsheet analysis`);
  console.log(`- visualization.html: Interactive visualization`);
  console.log('\nTop 10 smallest combinations:');
  
  results
    .sort((a, b) => a.wasmSize - b.wasmSize)
    .slice(0, 10)
    .forEach((r, i) => {
      console.log(`${(i + 1).toString().padStart(2)}. ${r.name.padEnd(20)} ${formatBytes(r.wasmSize).padStart(10)}`);
    });
    
  console.log(`\nOpen ${htmlPath} in a browser to view the interactive report.`);
}

// Run the analysis
main().catch(console.error);