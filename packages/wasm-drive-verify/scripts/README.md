# Module Size Analysis Scripts

This directory contains scripts for analyzing the bundle sizes of different module combinations in wasm-drive-verify.

## Available Scripts

### 1. `quick-size-check.sh`
Quick check of common module combinations. Run this for a fast overview.

```bash
./scripts/quick-size-check.sh
```

**Output**: Size comparison table showing the most common combinations and their reductions.

### 2. `analyze-module-combinations.sh`
Comprehensive bash script that builds predefined combinations and generates reports.

```bash
./scripts/analyze-module-combinations.sh
```

**Output**:
- `analysis-results/results.csv` - Raw size data
- `analysis-results/analysis-report.md` - Detailed markdown report
- `analysis-results/chart-data.json` - Data for visualization

### 3. `generate-size-matrix.js`
Node.js script that generates ALL possible combinations and creates an interactive HTML report.

```bash
node ./scripts/generate-size-matrix.js
```

**Output**:
- `size-analysis/visualization.html` - Interactive HTML report with charts
- `size-analysis/report.json` - Complete analysis data
- `size-analysis/results.csv` - CSV for spreadsheet analysis

### 4. `analyze-size-matrix.py`
Python script with advanced analysis and visualizations (requires matplotlib, seaborn, pandas).

```bash
# Install dependencies (optional, for visualizations)
pip install matplotlib seaborn pandas numpy

# Run analysis
python3 ./scripts/analyze-size-matrix.py
```

**Output**:
- `module-size-analysis/analysis_report.md` - Comprehensive markdown report
- `module-size-analysis/analysis_report.json` - Full analysis data
- `module-size-analysis/*.png` - Visualization plots (if dependencies installed)

### 5. `build-modules.sh`
Builds ES module wrappers for the package.

```bash
./scripts/build-modules.sh
```

### 6. `build-separate-modules.sh`
Builds separate WASM modules for each feature combination.

```bash
./scripts/build-separate-modules.sh
```

## Understanding the Results

### Module Sizes
Each module adds to the base size:
- **Base**: Core WASM runtime (~200KB)
- **Identity**: User/identity verification (~200KB)
- **Document**: Document queries and verification (~150KB)
- **Contract**: Smart contract verification (~100KB)
- **Tokens**: Token balance and info (~150KB)
- **Governance**: Voting and governance (~250KB)
- **Transitions**: State transition verification (~100KB)

### Size Reduction
The percentage reduction compared to the full bundle (which includes everything).

### Common Combinations

| Use Case | Modules | Typical Size | Reduction |
|----------|---------|--------------|-----------|
| Identity App | identity | ~400KB | ~84% |
| Document Storage | document, contract | ~450KB | ~82% |
| DeFi App | identity, tokens, contract | ~700KB | ~72% |
| Governance Platform | governance, identity | ~650KB | ~74% |
| Lightweight Client | identity, document | ~550KB | ~78% |

## Best Practices

1. **Start Small**: Begin with only the modules you need
2. **Measure Impact**: Run `quick-size-check.sh` after adding modules
3. **Use Dynamic Imports**: Load rarely-used modules on demand
4. **Monitor Growth**: Set up CI to track bundle size over time

## Interpreting Results

### Interaction Effects
When combining modules, the total size is usually less than the sum of individual modules due to:
- Shared dependencies
- Compiler optimizations
- Dead code elimination

### Optimization Potential
The scripts also test `wasm-opt` optimization, which typically reduces size by 10-20%.

## Continuous Monitoring

Add to your CI/CD pipeline:

```yaml
# Example GitHub Action
- name: Check Bundle Sizes
  run: |
    ./scripts/quick-size-check.sh > size-report.txt
    # Fail if any bundle exceeds threshold
    if grep -q "full.*3MB" size-report.txt; then
      echo "Bundle size exceeds threshold!"
      exit 1
    fi
```