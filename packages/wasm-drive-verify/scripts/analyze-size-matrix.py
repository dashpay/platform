#!/usr/bin/env python3
"""
Advanced size analysis for wasm-drive-verify modules.
Generates detailed reports and visualizations of module size combinations.
"""

import json
import os
import subprocess
import itertools
from pathlib import Path
import csv
from typing import Dict, List, Tuple, Set

# Try to import visualization libraries (optional)
try:
    import matplotlib.pyplot as plt
    import seaborn as sns
    import pandas as pd
    import numpy as np
    HAS_VISUALIZATION = True
except ImportError:
    HAS_VISUALIZATION = False
    print("Warning: matplotlib/seaborn/pandas not installed. Visualizations will be skipped.")
    print("Install with: pip install matplotlib seaborn pandas numpy")

class ModuleSizeAnalyzer:
    def __init__(self, project_root: Path):
        self.project_root = project_root
        self.features = ['identity', 'document', 'contract', 'tokens', 'governance', 'transitions']
        self.base_features = 'console_error_panic_hook'
        self.results = []
        
    def generate_combinations(self) -> List[Dict]:
        """Generate all possible feature combinations."""
        combinations = []
        
        # Base (no features)
        combinations.append({
            'name': 'base',
            'features': [],
            'feature_string': self.base_features
        })
        
        # All possible combinations
        for r in range(1, len(self.features) + 1):
            for combo in itertools.combinations(self.features, r):
                name = '-'.join(c[:3] for c in combo) if len(combo) > 1 else combo[0]
                combinations.append({
                    'name': name,
                    'features': list(combo),
                    'feature_string': f"{self.base_features},{','.join(combo)}"
                })
        
        # Full bundle
        combinations.append({
            'name': 'full',
            'features': ['full'],
            'feature_string': f"{self.base_features},full"
        })
        
        return combinations
    
    def build_combination(self, combo: Dict, output_dir: Path) -> Dict:
        """Build a specific feature combination and measure its size."""
        print(f"Building {combo['name']}...")
        
        build_dir = output_dir / combo['name']
        build_dir.mkdir(parents=True, exist_ok=True)
        
        try:
            # Build with cargo
            subprocess.run([
                'cargo', 'build',
                '--target', 'wasm32-unknown-unknown',
                '--release',
                '--no-default-features',
                '--features', combo['feature_string']
            ], cwd=self.project_root, check=True, capture_output=True)
            
            # Run wasm-bindgen
            wasm_path = self.project_root / '../../target/wasm32-unknown-unknown/release/wasm_drive_verify.wasm'
            subprocess.run([
                'wasm-bindgen',
                str(wasm_path),
                '--out-dir', str(build_dir),
                '--target', 'web',
                '--out-name', 'bundle'
            ], check=True, capture_output=True)
            
            # Get sizes
            wasm_size = (build_dir / 'bundle_bg.wasm').stat().st_size
            js_size = (build_dir / 'bundle.js').stat().st_size
            
            # Try optimization
            opt_size = wasm_size
            try:
                subprocess.run([
                    'wasm-opt', '-Oz',
                    str(build_dir / 'bundle_bg.wasm'),
                    '-o', str(build_dir / 'bundle_bg_opt.wasm')
                ], check=True, capture_output=True)
                opt_size = (build_dir / 'bundle_bg_opt.wasm').stat().st_size
            except:
                pass
            
            return {
                **combo,
                'wasm_size': wasm_size,
                'js_size': js_size,
                'optimized_size': opt_size,
                'total_size': wasm_size + js_size,
                'success': True
            }
            
        except Exception as e:
            print(f"  Failed: {e}")
            return {
                **combo,
                'wasm_size': 0,
                'js_size': 0,
                'optimized_size': 0,
                'total_size': 0,
                'success': False,
                'error': str(e)
            }
    
    def analyze_interactions(self) -> Dict:
        """Analyze feature interactions and dependencies."""
        interactions = {}
        
        # Find base sizes
        base_result = next(r for r in self.results if r['name'] == 'base')
        base_size = base_result['wasm_size']
        
        # Individual feature sizes
        individual_sizes = {}
        for feature in self.features:
            result = next((r for r in self.results if r['name'] == feature), None)
            if result:
                individual_sizes[feature] = result['wasm_size'] - base_size
        
        # Pairwise interactions
        for f1, f2 in itertools.combinations(self.features, 2):
            pair_result = next((r for r in self.results 
                              if set(r['features']) == {f1, f2}), None)
            if pair_result:
                expected = base_size + individual_sizes.get(f1, 0) + individual_sizes.get(f2, 0)
                actual = pair_result['wasm_size']
                interaction = actual - expected
                interactions[f"{f1}-{f2}"] = {
                    'expected': expected,
                    'actual': actual,
                    'interaction': interaction,
                    'interaction_pct': (interaction / expected * 100) if expected > 0 else 0
                }
        
        return interactions
    
    def generate_report(self, output_dir: Path):
        """Generate comprehensive analysis report."""
        report = {
            'summary': self._generate_summary(),
            'combinations': self.results,
            'interactions': self.analyze_interactions(),
            'recommendations': self._generate_recommendations()
        }
        
        # Save JSON report
        with open(output_dir / 'analysis_report.json', 'w') as f:
            json.dump(report, f, indent=2)
        
        # Generate markdown report
        self._generate_markdown_report(report, output_dir / 'analysis_report.md')
        
        # Generate CSV
        self._generate_csv(output_dir / 'results.csv')
        
        # Generate visualizations if available
        if HAS_VISUALIZATION:
            self._generate_visualizations(output_dir)
    
    def _generate_summary(self) -> Dict:
        """Generate summary statistics."""
        base_size = next(r for r in self.results if r['name'] == 'base')['wasm_size']
        full_size = next(r for r in self.results if r['name'] == 'full')['wasm_size']
        
        return {
            'total_combinations': len(self.results),
            'base_size': base_size,
            'full_size': full_size,
            'max_reduction_pct': ((full_size - base_size) / full_size * 100),
            'average_size': sum(r['wasm_size'] for r in self.results) / len(self.results),
            'smallest_useful': min((r for r in self.results if r['features']), 
                                 key=lambda x: x['wasm_size'])['name'],
            'most_efficient_combo': self._find_most_efficient_combo()
        }
    
    def _find_most_efficient_combo(self) -> str:
        """Find the combination with best size/feature ratio."""
        best_ratio = float('inf')
        best_combo = None
        
        for result in self.results:
            if result['features'] and result['wasm_size'] > 0:
                ratio = result['wasm_size'] / len(result['features'])
                if ratio < best_ratio:
                    best_ratio = ratio
                    best_combo = result['name']
        
        return best_combo
    
    def _generate_recommendations(self) -> List[Dict]:
        """Generate use-case based recommendations."""
        recommendations = []
        
        use_cases = [
            {
                'name': 'Minimal Identity Verification',
                'features': ['identity'],
                'description': 'Basic identity verification for wallets'
            },
            {
                'name': 'Document Management System',
                'features': ['document', 'contract'],
                'description': 'Document storage with contract validation'
            },
            {
                'name': 'DeFi Application',
                'features': ['identity', 'tokens', 'contract'],
                'description': 'Complete DeFi functionality'
            },
            {
                'name': 'Governance Platform',
                'features': ['governance', 'identity'],
                'description': 'Voting and governance features'
            },
            {
                'name': 'Lightweight Client',
                'features': ['identity', 'document'],
                'description': 'Mobile or resource-constrained environments'
            }
        ]
        
        for use_case in use_cases:
            result = next((r for r in self.results 
                          if set(r['features']) == set(use_case['features'])), None)
            if result:
                recommendations.append({
                    **use_case,
                    'size': result['wasm_size'],
                    'size_formatted': self._format_bytes(result['wasm_size']),
                    'reduction': self._calculate_reduction(result['wasm_size'])
                })
        
        return recommendations
    
    def _generate_markdown_report(self, report: Dict, output_path: Path):
        """Generate markdown report."""
        with open(output_path, 'w') as f:
            f.write("# WASM Drive Verify - Module Size Analysis Report\n\n")
            
            # Summary
            summary = report['summary']
            f.write("## Summary\n\n")
            f.write(f"- **Total Combinations Tested**: {summary['total_combinations']}\n")
            f.write(f"- **Base Size**: {self._format_bytes(summary['base_size'])}\n")
            f.write(f"- **Full Bundle Size**: {self._format_bytes(summary['full_size'])}\n")
            f.write(f"- **Maximum Possible Reduction**: {summary['max_reduction_pct']:.1f}%\n")
            f.write(f"- **Most Efficient Combination**: {summary['most_efficient_combo']}\n\n")
            
            # Top 10 Smallest
            f.write("## Top 10 Smallest Combinations\n\n")
            f.write("| Rank | Name | Features | Size | Reduction |\n")
            f.write("|------|------|----------|------|----------|\n")
            
            sorted_results = sorted(self.results, key=lambda x: x['wasm_size'])
            for i, result in enumerate(sorted_results[:10]):
                features = ', '.join(result['features']) or 'base only'
                size = self._format_bytes(result['wasm_size'])
                reduction = self._calculate_reduction(result['wasm_size'])
                f.write(f"| {i+1} | {result['name']} | {features} | {size} | {reduction} |\n")
            
            # Feature interactions
            f.write("\n## Feature Interactions\n\n")
            f.write("Shows how features interact when combined (negative = smaller than expected):\n\n")
            f.write("| Combination | Expected | Actual | Interaction | Impact |\n")
            f.write("|-------------|----------|--------|-------------|--------|\n")
            
            for name, data in sorted(report['interactions'].items(), 
                                    key=lambda x: x[1]['interaction']):
                f.write(f"| {name} | {self._format_bytes(data['expected'])} | ")
                f.write(f"{self._format_bytes(data['actual'])} | ")
                f.write(f"{self._format_bytes(abs(data['interaction']))} | ")
                f.write(f"{data['interaction_pct']:+.1f}% |\n")
            
            # Recommendations
            f.write("\n## Recommended Combinations\n\n")
            for rec in report['recommendations']:
                f.write(f"### {rec['name']}\n")
                f.write(f"- **Description**: {rec['description']}\n")
                f.write(f"- **Features**: {', '.join(rec['features'])}\n")
                f.write(f"- **Size**: {rec['size_formatted']} ({rec['reduction']} reduction)\n\n")
    
    def _generate_csv(self, output_path: Path):
        """Generate CSV file."""
        with open(output_path, 'w', newline='') as f:
            writer = csv.writer(f)
            writer.writerow(['name', 'features', 'feature_count', 'wasm_size', 
                           'js_size', 'optimized_size', 'total_size', 'reduction_pct'])
            
            full_size = next(r for r in self.results if r['name'] == 'full')['wasm_size']
            
            for result in sorted(self.results, key=lambda x: x['wasm_size']):
                reduction = ((full_size - result['wasm_size']) / full_size * 100) if result['name'] != 'full' else 0
                writer.writerow([
                    result['name'],
                    ','.join(result['features']),
                    len(result['features']),
                    result['wasm_size'],
                    result['js_size'],
                    result['optimized_size'],
                    result['total_size'],
                    f"{reduction:.1f}"
                ])
    
    def _generate_visualizations(self, output_dir: Path):
        """Generate visualization plots."""
        df = pd.DataFrame(self.results)
        df['feature_count'] = df['features'].apply(len)
        df['size_mb'] = df['wasm_size'] / (1024 * 1024)
        
        # 1. Size by feature count
        plt.figure(figsize=(10, 6))
        sns.boxplot(data=df, x='feature_count', y='size_mb')
        plt.title('Bundle Size Distribution by Feature Count')
        plt.xlabel('Number of Features')
        plt.ylabel('Size (MB)')
        plt.savefig(output_dir / 'size_by_feature_count.png', dpi=150, bbox_inches='tight')
        plt.close()
        
        # 2. Feature impact heatmap
        self._generate_feature_heatmap(output_dir)
        
        # 3. Size progression
        plt.figure(figsize=(12, 6))
        sorted_df = df.sort_values('wasm_size')
        plt.plot(range(len(sorted_df)), sorted_df['size_mb'], marker='o')
        plt.title('Bundle Size Progression')
        plt.xlabel('Combination Index (sorted by size)')
        plt.ylabel('Size (MB)')
        plt.grid(True, alpha=0.3)
        plt.savefig(output_dir / 'size_progression.png', dpi=150, bbox_inches='tight')
        plt.close()
    
    def _generate_feature_heatmap(self, output_dir: Path):
        """Generate heatmap of pairwise feature combinations."""
        # Create matrix
        matrix = np.zeros((len(self.features), len(self.features)))
        
        for i, f1 in enumerate(self.features):
            for j, f2 in enumerate(self.features):
                if i == j:
                    # Diagonal: individual feature size
                    result = next((r for r in self.results if r['features'] == [f1]), None)
                else:
                    # Off-diagonal: pairwise combination
                    result = next((r for r in self.results 
                                 if set(r['features']) == {f1, f2}), None)
                
                if result:
                    matrix[i, j] = result['wasm_size'] / 1024  # KB
        
        # Plot heatmap
        plt.figure(figsize=(10, 8))
        sns.heatmap(matrix, 
                   xticklabels=self.features,
                   yticklabels=self.features,
                   annot=True,
                   fmt='.0f',
                   cmap='YlOrRd',
                   cbar_kws={'label': 'Size (KB)'})
        plt.title('Feature Combination Sizes')
        plt.tight_layout()
        plt.savefig(output_dir / 'feature_heatmap.png', dpi=150, bbox_inches='tight')
        plt.close()
    
    def _format_bytes(self, bytes: int) -> str:
        """Format bytes to human readable string."""
        for unit in ['B', 'KB', 'MB', 'GB']:
            if bytes < 1024.0:
                return f"{bytes:.1f} {unit}"
            bytes /= 1024.0
        return f"{bytes:.1f} TB"
    
    def _calculate_reduction(self, size: int) -> str:
        """Calculate reduction percentage from full bundle."""
        full_size = next(r for r in self.results if r['name'] == 'full')['wasm_size']
        if size >= full_size:
            return "baseline"
        reduction = ((full_size - size) / full_size * 100)
        return f"{reduction:.1f}%"
    
    def run_analysis(self):
        """Run the complete analysis."""
        output_dir = self.project_root / 'module-size-analysis'
        output_dir.mkdir(exist_ok=True)
        
        # Clean previous results
        for f in output_dir.glob('*'):
            if f.is_file():
                f.unlink()
        
        # Generate combinations
        combinations = self.generate_combinations()
        print(f"Testing {len(combinations)} feature combinations...\n")
        
        # Build each combination
        for combo in combinations:
            result = self.build_combination(combo, output_dir / 'builds')
            self.results.append(result)
        
        print("\nAnalysis complete. Generating reports...")
        
        # Generate reports
        self.generate_report(output_dir)
        
        print(f"\nResults saved to: {output_dir}")
        print("- analysis_report.json: Complete analysis data")
        print("- analysis_report.md: Markdown report")
        print("- results.csv: CSV for spreadsheet analysis")
        
        if HAS_VISUALIZATION:
            print("- *.png: Visualization plots")


if __name__ == "__main__":
    # Find project root
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    
    # Run analysis
    analyzer = ModuleSizeAnalyzer(project_root)
    analyzer.run_analysis()