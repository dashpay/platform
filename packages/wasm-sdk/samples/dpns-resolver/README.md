# DPNS Resolver - Sample Application

A comprehensive DPNS (Dash Platform Name Service) application demonstrating username resolution, validation, registration cost calculation, and domain browsing using the Dash Platform WASM SDK.

## Features

### 🔍 Username Resolution
- **Forward Resolution**: Resolve usernames to identity IDs (alice.dash → identity)
- **Reverse Resolution**: Find usernames owned by an identity ID
- **Real-time Validation**: Instant feedback on username validity
- **Performance Tracking**: Resolution time monitoring and optimization

### ✅ Comprehensive Username Validation
- **Format Validation**: Length, character, and structure requirements
- **Availability Checking**: Network-based availability verification
- **Homograph Protection**: Detection and prevention of confusing characters
- **Quality Scoring**: Username quality assessment (0-100 scale)
- **Reserved Word Protection**: Prevention of system reserved names

### 💰 Registration Cost Calculator
- **Dynamic Pricing**: Length-based cost calculation
- **Quality Factors**: Premium pricing for high-value names
- **Real-time Updates**: Cost changes as you type
- **Detailed Breakdown**: Complete cost factor analysis

### 🌍 Domain Browser & Registry
- **Registry Exploration**: Browse registered domains with filtering
- **Advanced Filtering**: By length, owner, registration date
- **Domain Analytics**: Statistics and trends analysis
- **Export Capabilities**: Domain data export for analysis

### 📊 DPNS Network Statistics
- **Registration Metrics**: Total domains, recent activity
- **Length Distribution**: Average domain length analysis
- **Network Health**: Registration rate and activity monitoring
- **Contested Domains**: Dispute tracking and statistics

## Quick Start

### Prerequisites
- Modern web browser with WebAssembly support
- Local web server for WASM module loading
- Internet connection for Dash Platform network access

### Running the Application

1. **Start local web server** from the wasm-sdk directory:
   ```bash
   python3 -m http.server 8888
   ```

2. **Open in browser**:
   ```
   http://localhost:8888/samples/dpns-resolver/
   ```

3. **Try username resolution**:
   - Click "Try Sample" to load a known username
   - Or enter your own username to resolve
   - View detailed resolution results and identity information

## Usage Guide

### Username Resolution

#### Forward Resolution (Username → Identity)
1. Enter a username in the resolution field (e.g., "alice")
2. Click "Resolve" to find the associated identity
3. View complete resolution data including:
   - Target identity ID
   - Domain registration details
   - Resolution performance metrics

#### Reverse Resolution (Identity → Usernames)
1. Enter an identity ID in the reverse resolution field
2. Click "Find Username" to discover owned usernames
3. Browse all domains owned by that identity
4. Click individual usernames to resolve them forward

### Username Validation

#### Comprehensive Validation
1. Enter a username in the validation field
2. Select validation options:
   - ✅ **Check Availability**: Verify if username is available for registration
   - ✅ **Check Contested**: Check if username is in dispute
   - ✅ **Check Homograph**: Detect confusing character lookalikes
3. Click "Validate" to run complete validation
4. Review validation results including:
   - Format compliance
   - Network availability
   - Quality score
   - Registration cost estimate

#### Real-time Feedback
- **As-you-type validation**: Instant feedback while typing
- **Character validation**: Real-time checking of allowed characters
- **Length requirements**: Immediate length validation feedback
- **Quality suggestions**: Tips for improving username quality

### Registration Cost Calculation

1. Enter desired username in the registration calculator
2. View real-time cost factors:
   - **Length**: Character count and length-based multiplier
   - **Base Cost**: Standard registration fee
   - **Quality Multiplier**: Quality-based pricing adjustment
   - **Total Cost**: Final registration cost estimate

3. View detailed cost breakdown:
   - Cost in credits and DASH
   - Quality score impact
   - Length-based pricing tiers

### Domain Browsing

#### Filter Options
- **Recent Registrations**: Newest domains first
- **Short Names**: Domains ≤5 characters (premium names)
- **Long Names**: Domains >10 characters
- **By Owner**: Domains owned by specific identity

#### Browse Results
- **Domain Grid**: Visual display of domain information
- **Domain Details**: Click any domain for complete information
- **Export Options**: Download domain data for analysis

### Network Statistics

1. Click "Refresh Statistics" to get current DPNS metrics
2. View network health indicators:
   - Total domains in sample
   - Recent registrations (24h)
   - Average domain length
   - Contested domains count

## API Methods Demonstrated

This application showcases these WASM SDK DPNS methods:

```javascript
// Username validation (synchronous)
const isValid = sdk.dpns_is_valid_username(username);
const isContested = sdk.dpns_is_contested_username(username);
const homographSafe = sdk.dpns_convert_to_homograph_safe(username);

// Username resolution (asynchronous)
const identity = await sdk.dpns_resolve_name(username);
const isAvailable = await sdk.dpns_is_name_available(username);

// Domain queries via document system
const domains = await sdk.get_documents(
    dpnsContractId,
    'domain',
    JSON.stringify([['$ownerId', '=', identityId]]), // WHERE clause
    JSON.stringify([['$createdAt', 'desc']]),        // ORDER BY clause
    50,  // limit
    0    // offset
);

// Domain registration (state transition)
const registration = await sdk.dpns_register_name(
    username,
    identityId,
    publicKeyId,
    privateKeyWif,
    preorderCallback
);
```

## DPNS Validation Rules

### Username Requirements

#### Length
- **Minimum**: 3 characters
- **Maximum**: 63 characters
- **Optimal**: 6-12 characters for best cost/memorability ratio

#### Allowed Characters
- **Letters**: a-z, A-Z (case insensitive)
- **Numbers**: 0-9
- **Special**: . (period), - (hyphen), _ (underscore)
- **Forbidden**: Spaces, @, #, $, %, &, *, +, =, etc.

#### Format Rules
- Cannot start or end with . or -
- Cannot contain consecutive dots (..) or hyphens (--)
- Cannot be entirely numeric
- Case insensitive (Alice = alice = ALICE)

#### Reserved Words
The following are reserved and cannot be registered:
- `dash`, `admin`, `root`, `www`, `api`, `mail`, `ftp`
- `localhost`, `test`, `dev`, `staging`, `prod`, `production`

### Quality Factors

#### High Quality Names (Score 70-100)
- Pronounceable English words
- Common names or dictionary words
- Clean character composition
- Appropriate length (4-12 characters)

#### Medium Quality Names (Score 40-69)
- Mixed character types
- Some quality issues but still usable
- Longer names (13-20 characters)

#### Low Quality Names (Score 0-39)
- Entirely numeric
- Contains homograph characters
- Very long or confusing composition
- Invalid format or reserved words

### Registration Pricing Tiers

#### Premium Names (10x base cost)
- **Length**: 1-4 characters
- **Examples**: `bob`, `pay`, `dev`, `app`
- **Target Market**: Businesses, brands, premium users

#### High-Value Names (3x base cost)
- **Length**: 5-6 characters  
- **Examples**: `alice`, `token`, `trade`
- **Target Market**: Personal brands, common words

#### Standard Names (1.5x base cost)
- **Length**: 7-10 characters
- **Examples**: `myusername`, `dashboard`
- **Target Market**: General users, descriptive names

#### Economy Names (1x base cost)
- **Length**: 11+ characters
- **Examples**: `verylongusername`, `mycomplexidentifier`
- **Target Market**: Cost-conscious users, specialized applications

## Sample Data & Testing

### Known Test Usernames (Testnet)
- **alice**: Sample user for testing resolution
- **bob**: Alternative test username
- **test123**: Example of numeric username

### Test Identity IDs (Testnet)
- **Sample Identity**: `5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk`
- **Use for reverse resolution testing**

### Testing Scenarios

#### Resolution Testing
1. **Existing Username**: Try "alice" or other known usernames
2. **Non-existent Username**: Try "thisusernameprobaблydoesnotexist123"
3. **Invalid Username**: Try "user@name" or other invalid formats

#### Validation Testing  
1. **Valid Username**: "johndoe" (should pass all checks)
2. **Too Short**: "ab" (should fail length check)
3. **Invalid Characters**: "user@name" (should fail character check)
4. **Reserved Word**: "admin" (should fail reserved check)
5. **Homograph**: "аlice" (Cyrillic 'а' instead of Latin 'a')

#### Cost Calculation Testing
1. **Premium Name**: "bob" (4 chars, should be 10x base cost)
2. **Standard Name**: "username" (8 chars, should be 1.5x base cost)
3. **Economy Name**: "verylongusername" (16 chars, should be 1x base cost)

## Performance Characteristics

### Operation Timing Expectations
- **Username Resolution**: 200-800ms
- **Domain Queries**: 300ms-2s (depending on result size)
- **Validation**: 50-200ms for network checks
- **Statistics**: 1-3s for comprehensive stats

### Memory Usage
- **Base Application**: ~6-8MB
- **Domain Cache**: ~2KB per domain
- **Statistics**: ~5MB for large registry samples

### Network Efficiency
- **Query Optimization**: Intelligent caching and batching
- **Progressive Loading**: Incremental data loading
- **Error Resilience**: Graceful degradation on network issues

## Data Export Formats

### Resolution Data Export
```javascript
{
  "username": "alice.dash",
  "resolvedTo": "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF",
  "network": "testnet",
  "contractId": "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
  "resolvedAt": "2025-09-04T10:30:00Z",
  "resolutionTime": 340
}
```

### Validation Report Export
```javascript
{
  "validation": {
    "username": "alice",
    "valid": true,
    "qualityScore": 85,
    "errors": [],
    "warnings": []
  },
  "network": {
    "available": false,
    "contested": false,
    "homographSafe": true
  },
  "estimatedCost": {
    "total": 3000000,
    "baseCost": 1000000,
    "lengthMultiplier": 3.0
  }
}
```

### Domain Registry Export
```javascript
{
  "domains": [
    {
      "label": "alice",
      "ownerId": "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF",
      "createdAt": 1640995200000,
      "records": { /* DNS-like records */ }
    }
  ],
  "metadata": {
    "totalDomains": 150,
    "filter": "recent",
    "network": "testnet"
  }
}
```

## Browser Compatibility

- **Chrome 80+**: Full support with optimal performance
- **Firefox 75+**: Full support with good performance
- **Safari 13+**: Full support (HTTPS recommended for production)
- **Edge 80+**: Full support with optimal performance

## Security & Privacy

### Data Handling
- **No Private Data Storage**: Application doesn't store private keys or sensitive data
- **Read-only Operations**: Only performs queries, no write operations by default
- **Network Security**: All communications over HTTPS
- **Local Processing**: Validation and cost calculation performed locally

### Registration Security
- **Demo Mode**: Registration functionality is demonstration only
- **Private Key Warning**: Clear warnings about private key usage
- **Testnet Recommendation**: Encourage testnet testing before mainnet use

## Troubleshooting

### Common Issues

**Username Not Resolving**
- Verify username is spelled correctly
- Check that you're on the correct network (testnet vs mainnet)
- Try known test usernames like "alice" on testnet

**Validation Errors**
- Review character requirements (only a-z, 0-9, ., -, _)
- Check length requirements (3-63 characters)
- Avoid reserved words and homograph characters

**Domain Browsing Issues**
- Large queries may take time - try reducing limit
- Some filters require specific query indexes
- Network connectivity may affect browsing performance

**Cost Calculation Issues**
- Costs are estimates and may vary from actual network costs
- Very short names (1-4 chars) have premium pricing
- Quality factors affect final pricing

### Debug Mode

Enable debug mode by clicking "Toggle Debug" in the operations log:
- **Enhanced Logging**: Detailed operation traces
- **Performance Metrics**: Query timing and optimization data
- **Error Context**: Full error details and stack traces
- **Network Monitoring**: Connection health and endpoint status

### Known Limitations

**Network Dependencies**
- Some features require active network connection
- Testnet may have different data than mainnet
- Registration preview only - actual registration requires state transitions

**Query Limitations**
- Domain filtering by length requires client-side processing
- Complex queries may timeout on large datasets
- Some advanced DPNS features may not be available in WASM SDK

## Integration Examples

### Using DPNS Resolution in Your App

```javascript
import { DPNSResolver } from './dpns-resolver/app.js';

// Initialize resolver
const resolver = new DPNSResolver();
await resolver.init();

// Resolve a username
const identity = await resolver.resolveUsername('alice');

// Validate a username
const validation = await resolver.validateUsername('newusername');

// Calculate registration cost
const cost = resolver.calculateRegistrationCost('newusername');
```

### Building Custom DPNS Interfaces

```javascript
// Basic username resolution
const identity = await sdk.dpns_resolve_name('alice');

// Username validation
const isValid = sdk.dpns_is_valid_username('alice');
const isAvailable = await sdk.dpns_is_name_available('alice');
const isContested = sdk.dpns_is_contested_username('alice');

// Homograph safety
const safeName = sdk.dpns_convert_to_homograph_safe('аlice'); // Converts Cyrillic 'а' to Latin 'a'

// Domain queries
const domains = await sdk.get_documents(
    dpnsContractId,
    'domain',
    JSON.stringify([['$ownerId', '=', identityId]]),
    JSON.stringify([['$createdAt', 'desc']]),
    10,
    0
);
```

### Validation Integration

```javascript
import { DPNSValidator } from './dpns-resolver/validation.js';

const validator = new DPNSValidator();

// Comprehensive validation
const validation = validator.validateUsername('alice');

// Real-time validation  
const realtimeCheck = validator.validateAsYouType('ali'); // Partial input

// Cost estimation
const cost = validator.estimateRegistrationCost('alice');

// Quality scoring
const score = validator.calculateQualityScore('alice'); // 0-100

// Generate alternatives
const suggestions = validator.generateSuggestions('alice', 5);
```

## DPNS Protocol Deep Dive

### Domain Document Structure
```javascript
{
  "label": "alice",                    // Username without .dash
  "normalizedLabel": "alice",          // Normalized version  
  "normalizedParentDomainName": "dash", // Parent domain
  "preorderSalt": [/* bytes */],       // Registration salt
  "records": {                         // DNS-like records
    "identity": "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF"
  },
  "subdomainRules": {                  // Subdomain policies
    "allowSubdomains": false
  }
}
```

### Registration Process Flow
1. **Username Validation**: Check format and availability
2. **Preorder Creation**: Submit salted hash of desired name
3. **Reveal Period**: Wait for confirmation period
4. **Domain Registration**: Submit actual domain registration
5. **Confirmation**: Verify registration success

### Contest Resolution
- **Contest Period**: Time window for disputing registrations
- **Evidence Submission**: Provide evidence for ownership claims
- **Community Voting**: Masternode voting on disputes
- **Resolution**: Final ownership determination

## Advanced Features

### Batch Operations
```javascript
// Check multiple usernames
const usernames = ['alice', 'bob', 'charlie'];
const validations = await Promise.all(
    usernames.map(u => resolver.validateUsername(u))
);

// Reverse resolve multiple identities
const identities = ['id1', 'id2', 'id3'];
const allDomains = await Promise.all(
    identities.map(id => resolver.reverseResolve(id))
);
```

### Custom Validation Rules
```javascript
// Extend validation with custom rules
class CustomDPNSValidator extends DPNSValidator {
    validateUsername(username) {
        const baseValidation = super.validateUsername(username);
        
        // Add custom business rules
        if (username.includes('test')) {
            baseValidation.warnings.push('Username contains "test" - may be temporary');
        }
        
        return baseValidation;
    }
}
```

### Analytics Integration
```javascript
// Track resolution patterns
const analytics = {
    resolutions: 0,
    averageTime: 0,
    successRate: 0,
    popularNames: new Map()
};

// Monitor usage patterns
resolver.on('resolution', (data) => {
    analytics.resolutions++;
    analytics.averageTime = (analytics.averageTime + data.time) / 2;
    analytics.popularNames.set(data.username, 
        (analytics.popularNames.get(data.username) || 0) + 1);
});
```

This comprehensive DPNS sample application demonstrates the full capabilities of Dash Platform's naming service while providing practical examples for integration into your own applications.