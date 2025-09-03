---
name: WASM SDK Performance Issue
about: Report performance problems with the @dashevo/dash-wasm-sdk package
title: '[WASM SDK Performance] '
labels: 'performance, wasm-sdk'
assignees: ''

---

**Performance Issue Description**
A clear description of the performance problem you're experiencing.

**Affected Operations**
Which SDK operations are performing slowly?
- [ ] Identity operations (create, topup, etc.)
- [ ] Document operations (query, create, update)
- [ ] Contract operations
- [ ] Token operations
- [ ] WASM initialization/loading
- [ ] Memory usage
- [ ] Other: ___________

**Performance Measurements**
Please provide specific timing data if available:

```
Operation: identity creation
Expected time: < 1 second
Actual time: 5.2 seconds
Memory usage: 150MB peak

Operation: document query
Expected time: < 500ms  
Actual time: 3.1 seconds
```

**Environment Details**
- OS: [e.g. Windows 11, macOS 14, Ubuntu 22.04]
- Runtime: [e.g. Chrome 120, Node.js 20.10, Safari 17]
- Package Version: [e.g. 0.1.0-alpha.1] 
- Network: [testnet/mainnet]
- Hardware: [CPU model, RAM amount if relevant]

**Code Sample**
```javascript
// Code that demonstrates the performance issue
import initWasm, { WasmSdkBuilder } from '@dashevo/dash-wasm-sdk';

// Your performance test code here
const startTime = performance.now();
// ... operation ...
const endTime = performance.now();
console.log(`Operation took ${endTime - startTime} milliseconds`);
```

**Expected Performance**
What performance characteristics would you expect?
- Response time targets
- Memory usage expectations
- Comparison with other similar operations

**Impact Assessment**
How does this performance issue affect your application?
- [ ] Blocking - makes the application unusable
- [ ] Severe - significantly degrades user experience  
- [ ] Moderate - causes noticeable delays
- [ ] Minor - slight performance impact

**Additional Context**
- Bundle size impact
- Network conditions during testing
- Device specifications
- Comparison with previous versions
- Any optimization attempts you've tried