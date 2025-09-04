# Document Explorer - Sample Application

A comprehensive document query application demonstrating advanced Dash Platform WASM SDK document operations with filtering, sorting, and export capabilities.

## Features

### 📋 Smart Contract Integration
- **System Contracts**: Pre-configured DPNS, DashPay, and Withdrawals contracts
- **Custom Contracts**: Load any contract by ID with automatic schema inference
- **Contract Visualization**: View contract details, version, and document types
- **Network Support**: Full testnet and mainnet compatibility

### 🔍 Advanced Query Builder
- **Visual Query Builder**: No-code interface for complex queries
- **WHERE Clauses**: Support for all operators (=, !=, >, <, >=, <=, in, startsWith)
- **ORDER BY Clauses**: Multi-field sorting with ascending/descending options
- **Field Validation**: Smart field suggestions based on document schemas
- **Sample Queries**: Pre-built queries for common use cases

### 📊 Rich Results Display
- **Grid View**: Document cards with key metadata
- **Detailed Modal**: Full document inspection with formatted JSON
- **Performance Metrics**: Query execution time tracking
- **Real-time Updates**: Dynamic result rendering

### 📈 Data Export & Analysis
- **JSON Export**: Complete results with metadata
- **CSV Export**: Spreadsheet-compatible format with flattened data
- **Query Export**: Copy-paste ready code for integration
- **Single Document Export**: Individual document extraction

### 🕒 Query History & Analytics
- **Operation Logging**: Complete audit trail of all queries
- **Performance Tracking**: Query time analysis across operations
- **History Export**: Downloadable query history for analysis
- **Success Rate Monitoring**: Track query success patterns

## Quick Start

### Prerequisites
- Modern web browser with WebAssembly support
- Local web server for WASM module loading

### Running the Application

1. **Start local web server** from the wasm-sdk directory:
   ```bash
   python3 -m http.server 8888
   ```

2. **Open in browser**:
   ```
   http://localhost:8888/samples/document-explorer/
   ```

3. **Explore system contracts**:
   - Click "DPNS" to load the Domain Name Service contract
   - Select "domain" document type to see username registrations
   - Click "🚀 Execute Query" to see recent domain registrations

## Usage Guide

### Basic Document Query

1. **Select a Contract**:
   - Click a system contract button (DPNS, DashPay, Withdrawals)
   - Or enter a custom contract ID and click "Load Contract"

2. **Choose Document Type**:
   - Select from the dropdown or click a document type button
   - Fields will auto-populate based on document schema

3. **Build Your Query** (Optional):
   - Add WHERE conditions for filtering
   - Add ORDER BY clauses for sorting
   - Adjust limit and offset for pagination

4. **Execute**:
   - Click "🚀 Execute Query" to run the query
   - Results appear below with performance metrics

### Advanced Query Examples

#### Finding Specific User Profiles
```javascript
// Query: DashPay profiles with display names
Contract: DashPay
Document Type: profile
WHERE: displayName != null
ORDER BY: $updatedAt desc
LIMIT: 20
```

#### Domain Name Analysis
```javascript
// Query: Short domain names
Contract: DPNS  
Document Type: domain
WHERE: label length < 5
ORDER BY: label asc
LIMIT: 50
```

#### Recent Platform Activity
```javascript
// Query: Recent withdrawals over 1 DASH
Contract: Withdrawals
Document Type: withdrawal
WHERE: amount > 100000000, $createdAt > 1640995200000
ORDER BY: $createdAt desc
LIMIT: 10
```

## API Methods Demonstrated

This application showcases these WASM SDK methods:

```javascript
// Data contract operations
await sdk.get_data_contract(contractId)

// Document querying with advanced parameters
await sdk.get_documents(
    contractId,
    documentType,
    whereClause,     // JSON string: [["field", "operator", "value"]]
    orderByClause,   // JSON string: [["field", "direction"]]
    limit,           // Number of results
    offset           // Results to skip
)
```

### WHERE Clause Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `=` | Equal | `["displayName", "=", "Alice"]` |
| `!=` | Not equal | `["status", "!=", 0]` |
| `>` | Greater than | `["amount", ">", 1000000]` |
| `>=` | Greater or equal | `["$createdAt", ">=", 1640995200000]` |
| `<` | Less than | `["label", "length", "<", 5]` |
| `<=` | Less or equal | `["balance", "<=", 100000000]` |
| `in` | In array | `["status", "in", [1, 2, 3]]` |
| `startsWith` | String prefix | `["label", "startsWith", "test"]` |

### ORDER BY Directions

| Direction | Description |
|-----------|-------------|
| `asc` | Ascending (A-Z, 1-9, oldest first) |
| `desc` | Descending (Z-A, 9-1, newest first) |

## System Contract Reference

### DPNS (Domain Name Service)
- **Purpose**: Username and domain management
- **Documents**: `domain`, `preorder`
- **Common Queries**: Recent registrations, domains by owner, short names

### DashPay (Social Payments)
- **Purpose**: Social payment features and user profiles
- **Documents**: `profile`, `contactInfo`, `contactRequest`
- **Common Queries**: User profiles, contact networks, recent activity

### Withdrawals
- **Purpose**: Platform credit withdrawals to Layer 1
- **Documents**: `withdrawal`
- **Common Queries**: Recent withdrawals, large amounts, withdrawal status

## Performance Characteristics

### Query Performance Expectations
- **Simple Queries**: 100-500ms (identity-based filtering)
- **Complex Queries**: 500ms-2s (multiple conditions, large result sets)
- **Network Impact**: Testnet typically faster than mainnet

### Memory Usage
- **Base Application**: ~5-10MB
- **Per Query Result**: ~1KB per document
- **Large Result Sets**: Up to 100MB for 10,000+ documents

### Optimization Tips
- Use specific WHERE conditions to limit result sets
- Implement pagination with LIMIT and OFFSET
- Cache frequently accessed contracts and schemas
- Monitor query performance in the operations log

## Data Export Formats

### JSON Export Structure
```javascript
{
  "query": {
    "contract": "DPNS",
    "contractId": "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
    "documentType": "domain",
    "parameters": { /* query params */ }
  },
  "results": [ /* document array */ ],
  "metadata": {
    "resultCount": 25,
    "queryTime": 340,
    "exportedAt": "2025-09-04T10:30:00Z",
    "network": "testnet"
  }
}
```

### CSV Export Features
- **Flattened Structure**: Nested objects flattened to dot notation
- **Data Fields**: All document.data fields exported as separate columns
- **System Fields**: $ownerId, $createdAt, $updatedAt included
- **Excel Compatible**: Standard CSV format for spreadsheet applications

## Browser Compatibility

- **Chrome 80+**: Full support with optimal performance
- **Firefox 75+**: Full support with good performance
- **Safari 13+**: Full support (may require HTTPS in production)
- **Edge 80+**: Full support with optimal performance

## Security Considerations

- **Read-Only Operations**: This application only performs queries, no write operations
- **No Private Data**: No private keys or sensitive information stored
- **Network Security**: All communications use HTTPS endpoints
- **Data Privacy**: Query history stored only in browser session

## Troubleshooting

### Common Issues

**Contract Not Loading**
- Verify contract ID is correct and Base58 encoded
- Check network selection (testnet vs mainnet)
- Some contracts may not exist on all networks

**Query Returns No Results**
- Try removing WHERE conditions
- Check field names and values
- Verify document type exists in contract
- Use sample queries for known working examples

**Performance Issues**
- Reduce LIMIT value for large queries
- Add specific WHERE conditions to narrow results
- Monitor query time in results header
- Consider pagination for large datasets

**WASM Loading Errors**
- Ensure running from web server (not file://)
- Check browser console for detailed errors
- Verify WASM files exist in ../../pkg/ directory

### Debug Mode

Enable detailed logging by opening browser developer tools. The application provides:

- **Console Logging**: All operations logged with timestamps
- **Error Context**: Detailed error information with stack traces
- **Performance Metrics**: Query timing and resource usage
- **Network Monitoring**: SDK connection status and health

## Integration Examples

### Using Query Logic in Your App

```javascript
import { DocumentExplorer } from './document-explorer/app.js';

// Initialize explorer
const explorer = new DocumentExplorer();
await explorer.init();

// Load a specific contract
await explorer.loadKnownContract('dpns');

// Execute custom queries
const results = await explorer.executeCustomQuery(
    'domain',
    [['$ownerId', '=', 'your-identity-id']],
    [['$createdAt', 'desc']],
    10
);
```

### Building Custom Query Interfaces

```javascript
// Use the query builder logic
const params = {
    where: [['field', 'operator', 'value']],
    orderBy: [['field', 'direction']],
    limit: 10,
    offset: 0
};

const results = await sdk.get_documents(
    contractId,
    documentType,
    JSON.stringify(params.where),
    JSON.stringify(params.orderBy),
    params.limit,
    params.offset
);
```

This sample application serves as both a practical tool for exploring Dash Platform data and a comprehensive reference for implementing document query functionality in your own applications.