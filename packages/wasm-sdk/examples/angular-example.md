# Angular Integration Example

This guide demonstrates how to integrate the Dash Platform WASM SDK into an Angular application using the modern WasmSDK wrapper with proper dependency injection and reactive patterns.

## Table of Contents

- [Installation](#installation)
- [Injectable Service](#injectable-service)
- [Module Configuration](#module-configuration)
- [Component Examples](#component-examples)
- [RxJS Integration](#rxjs-integration)
- [State Management with NgRx](#state-management-with-ngrx)
- [Standalone Components](#standalone-components)
- [Complete Dashboard Example](#complete-dashboard-example)
- [TypeScript Integration](#typescript-integration)
- [Testing](#testing)
- [Troubleshooting](#troubleshooting)

## Installation

```bash
# Angular with WASM SDK
npm install @dashevo/dash-wasm-sdk @angular/core @angular/common

# RxJS for reactive programming
npm install rxjs

# Optional: NgRx for state management
npm install @ngrx/store @ngrx/effects @ngrx/entity

# Development dependencies
npm install --save-dev @angular/cli @types/node
```

## Injectable Service

### Basic WASM SDK Service

```typescript
// services/wasm-sdk.service.ts
import { Injectable, OnDestroy } from '@angular/core';
import { BehaviorSubject, Observable, from, throwError, timer } from 'rxjs';
import { catchError, retry, switchMap, tap } from 'rxjs/operators';
import { WasmSDK, WasmSDKConfig, WasmTransportError } from '@dashevo/dash-wasm-sdk';

export interface SDKState {
  sdk: WasmSDK | null;
  loading: boolean;
  error: Error | null;
  initialized: boolean;
  connected: boolean;
}

@Injectable({
  providedIn: 'root'
})
export class WasmSDKService implements OnDestroy {
  private sdkInstance: WasmSDK | null = null;
  private readonly _state$ = new BehaviorSubject<SDKState>({
    sdk: null,
    loading: false,
    error: null,
    initialized: false,
    connected: false
  });

  public readonly state$ = this._state$.asObservable();
  public readonly sdk$ = this.state$.pipe(
    map(state => state.sdk),
    filter((sdk): sdk is WasmSDK => sdk !== null)
  );

  private config: WasmSDKConfig = {
    network: 'testnet',
    transport: {
      url: 'https://52.12.176.90:1443/',
      timeout: 30000,
      retries: 3
    },
    proofs: true,
    debug: false
  };

  constructor() {}

  async initialize(customConfig?: Partial<WasmSDKConfig>): Promise<void> {
    if (this.sdkInstance && this._state$.value.initialized) {
      return;
    }

    this.updateState({ loading: true, error: null });

    try {
      const config = { ...this.config, ...customConfig };
      this.sdkInstance = new WasmSDK(config);
      await this.sdkInstance.initialize();

      this.updateState({
        sdk: this.sdkInstance,
        loading: false,
        initialized: true,
        connected: true
      });
    } catch (error) {
      this.updateState({
        loading: false,
        error: error as Error,
        connected: false
      });
      throw error;
    }
  }

  async destroy(): Promise<void> {
    if (this.sdkInstance) {
      await this.sdkInstance.destroy();
      this.sdkInstance = null;
    }

    this.updateState({
      sdk: null,
      loading: false,
      error: null,
      initialized: false,
      connected: false
    });
  }

  // Query operations
  getIdentity(identityId: string): Observable<any> {
    return this.sdk$.pipe(
      switchMap(sdk => from(sdk.getIdentity(identityId))),
      retry(2),
      catchError(error => {
        console.error('Failed to get identity:', error);
        return throwError(() => error);
      })
    );
  }

  getDocuments(contractId: string, documentType: string, options: any = {}): Observable<any[]> {
    return this.sdk$.pipe(
      switchMap(sdk => from(sdk.getDocuments(contractId, documentType, options))),
      retry(2),
      catchError(error => {
        console.error('Failed to get documents:', error);
        return throwError(() => error);
      })
    );
  }

  getDataContract(contractId: string): Observable<any> {
    return this.sdk$.pipe(
      switchMap(sdk => from(sdk.getDataContract(contractId))),
      retry(2),
      catchError(error => {
        console.error('Failed to get data contract:', error);
        return throwError(() => error);
      })
    );
  }

  ngOnDestroy(): void {
    this.destroy();
  }

  private updateState(partialState: Partial<SDKState>): void {
    this._state$.next({
      ...this._state$.value,
      ...partialState
    });
  }
}
```

### Advanced Service with Retry Logic

```typescript
// services/wasm-sdk-advanced.service.ts
import { Injectable, OnDestroy, inject } from '@angular/core';
import { BehaviorSubject, Observable, from, throwError, timer, EMPTY } from 'rxjs';
import { 
  catchError, 
  retry, 
  retryWhen, 
  switchMap, 
  tap, 
  delay, 
  take, 
  concatMap,
  shareReplay 
} from 'rxjs/operators';
import { WasmSDK, WasmTransportError, WasmOperationError } from '@dashevo/dash-wasm-sdk';

interface RetryConfig {
  maxRetries: number;
  baseDelay: number;
  maxDelay: number;
}

@Injectable({
  providedIn: 'root'
})
export class WasmSDKAdvancedService implements OnDestroy {
  private sdkInstance: WasmSDK | null = null;
  private readonly _state$ = new BehaviorSubject<SDKState>({
    sdk: null,
    loading: false,
    error: null,
    initialized: false,
    connected: false
  });

  private readonly retryConfig: RetryConfig = {
    maxRetries: 3,
    baseDelay: 1000,
    maxDelay: 10000
  };

  public readonly state$ = this._state$.asObservable().pipe(shareReplay(1));
  
  public readonly sdk$ = this.state$.pipe(
    map(state => state.sdk),
    filter((sdk): sdk is WasmSDK => sdk !== null && state.initialized),
    shareReplay(1)
  );

  public readonly isReady$ = this.state$.pipe(
    map(state => state.initialized && !state.loading && !state.error),
    shareReplay(1)
  );

  constructor() {}

  initializeWithRetry(config?: Partial<WasmSDKConfig>): Observable<void> {
    return from(this.initialize(config)).pipe(
      retryWhen(errors =>
        errors.pipe(
          concatMap((error, index) => {
            if (index >= this.retryConfig.maxRetries) {
              return throwError(() => error);
            }

            const shouldRetry = error instanceof WasmTransportError;
            if (!shouldRetry) {
              return throwError(() => error);
            }

            const delayTime = Math.min(
              this.retryConfig.baseDelay * Math.pow(2, index),
              this.retryConfig.maxDelay
            );

            console.warn(`SDK initialization failed, retrying in ${delayTime}ms...`, error);
            return timer(delayTime);
          })
        )
      ),
      tap(() => console.log('SDK initialized successfully'))
    );
  }

  async initialize(customConfig?: Partial<WasmSDKConfig>): Promise<void> {
    if (this.sdkInstance && this._state$.value.initialized) {
      return;
    }

    this.updateState({ loading: true, error: null });

    try {
      const config = {
        network: 'testnet',
        transport: {
          url: 'https://52.12.176.90:1443/',
          timeout: 30000,
          retries: 3
        },
        proofs: true,
        debug: false,
        ...customConfig
      };

      this.sdkInstance = new WasmSDK(config);
      await this.sdkInstance.initialize();

      this.updateState({
        sdk: this.sdkInstance,
        loading: false,
        initialized: true,
        connected: true
      });
    } catch (error) {
      this.updateState({
        loading: false,
        error: error as Error,
        connected: false
      });
      throw error;
    }
  }

  // Enhanced query operations with better error handling
  getIdentityWithFallback(identityId: string): Observable<any> {
    return this.sdk$.pipe(
      switchMap(sdk => from(sdk.getIdentity(identityId))),
      catchError(error => {
        if (error instanceof WasmOperationError) {
          console.warn(`Identity ${identityId} not found or inaccessible`);
          return EMPTY; // Return empty instead of error for not found
        }
        return throwError(() => error);
      })
    );
  }

  getDocumentsPaginated(
    contractId: string, 
    documentType: string, 
    page: number = 0, 
    pageSize: number = 10
  ): Observable<{ documents: any[], hasMore: boolean }> {
    return this.sdk$.pipe(
      switchMap(sdk => {
        const options = {
          limit: pageSize,
          offset: page * pageSize
        };
        return from(sdk.getDocuments(contractId, documentType, options));
      }),
      map(documents => ({
        documents,
        hasMore: documents.length === pageSize
      })),
      catchError(error => {
        console.error('Failed to get paginated documents:', error);
        return throwError(() => error);
      })
    );
  }

  // Resource management
  getResourceStats(): Observable<any> {
    return this.sdk$.pipe(
      map(sdk => sdk.getResourceStats()),
      catchError(error => {
        console.error('Failed to get resource stats:', error);
        return throwError(() => error);
      })
    );
  }

  cleanupResources(): Observable<number> {
    return this.sdk$.pipe(
      map(sdk => sdk.cleanupResources()),
      tap(cleaned => console.log(`Cleaned up ${cleaned} resources`)),
      catchError(error => {
        console.error('Failed to cleanup resources:', error);
        return throwError(() => error);
      })
    );
  }

  ngOnDestroy(): void {
    this.destroy();
  }

  private async destroy(): Promise<void> {
    if (this.sdkInstance) {
      await this.sdkInstance.destroy();
      this.sdkInstance = null;
    }

    this.updateState({
      sdk: null,
      loading: false,
      error: null,
      initialized: false,
      connected: false
    });
  }

  private updateState(partialState: Partial<SDKState>): void {
    this._state$.next({
      ...this._state$.value,
      ...partialState
    });
  }
}
```

## Module Configuration

### SDK Module

```typescript
// modules/wasm-sdk.module.ts
import { NgModule, ModuleWithProviders } from '@angular/core';
import { CommonModule } from '@angular/common';
import { WasmSDKService } from '../services/wasm-sdk.service';
import { WasmSDKAdvancedService } from '../services/wasm-sdk-advanced.service';
import { WasmSDKConfig } from '@dashevo/dash-wasm-sdk';

export interface WasmSDKModuleConfig {
  config?: Partial<WasmSDKConfig>;
  useAdvanced?: boolean;
}

@NgModule({
  imports: [CommonModule],
  providers: [
    WasmSDKService,
    WasmSDKAdvancedService
  ]
})
export class WasmSDKModule {
  static forRoot(moduleConfig?: WasmSDKModuleConfig): ModuleWithProviders<WasmSDKModule> {
    return {
      ngModule: WasmSDKModule,
      providers: [
        {
          provide: 'WASM_SDK_CONFIG',
          useValue: moduleConfig?.config || {}
        },
        {
          provide: 'USE_ADVANCED_SDK',
          useValue: moduleConfig?.useAdvanced || false
        },
        WasmSDKService,
        WasmSDKAdvancedService
      ]
    };
  }

  static forFeature(): ModuleWithProviders<WasmSDKModule> {
    return {
      ngModule: WasmSDKModule
    };
  }
}
```

### App Module Setup

```typescript
// app.module.ts
import { NgModule } from '@angular/core';
import { BrowserModule } from '@angular/platform-browser';
import { ReactiveFormsModule } from '@angular/forms';
import { HttpClientModule } from '@angular/common/http';

import { AppRoutingModule } from './app-routing.module';
import { AppComponent } from './app.component';
import { WasmSDKModule } from './modules/wasm-sdk.module';

@NgModule({
  declarations: [
    AppComponent
  ],
  imports: [
    BrowserModule,
    ReactiveFormsModule,
    HttpClientModule,
    AppRoutingModule,
    WasmSDKModule.forRoot({
      config: {
        network: 'testnet',
        transport: {
          url: 'https://52.12.176.90:1443/',
          timeout: 30000
        },
        proofs: true,
        debug: false
      },
      useAdvanced: true
    })
  ],
  providers: [],
  bootstrap: [AppComponent]
})
export class AppModule { }
```

## Component Examples

### Identity Display Component

```typescript
// components/identity-display.component.ts
import { Component, Input, OnInit, OnDestroy, inject } from '@angular/core';
import { Observable, Subject, BehaviorSubject } from 'rxjs';
import { takeUntil, switchMap, catchError, finalize } from 'rxjs/operators';
import { WasmSDKAdvancedService } from '../services/wasm-sdk-advanced.service';

interface IdentityDisplayState {
  identity: any | null;
  loading: boolean;
  error: Error | null;
}

@Component({
  selector: 'app-identity-display',
  template: `
    <div class="identity-display">
      <h3>Identity Display</h3>
      
      <div *ngIf="state$ | async as state">
        <div *ngIf="state.loading" class="loading">
          <mat-spinner diameter="30"></mat-spinner>
          Loading identity...
        </div>

        <div *ngIf="state.error && !state.loading" class="error">
          <h4>Error loading identity</h4>
          <p>{{ state.error.message }}</p>
          <button mat-raised-button color="primary" (click)="refresh()">
            Retry
          </button>
        </div>

        <div *ngIf="state.identity && !state.loading" class="identity-details">
          <div class="identity-field">
            <label>ID:</label>
            <span class="monospace">{{ identityId }}</span>
          </div>
          
          <div class="identity-field">
            <label>Public Keys:</label>
            <span>{{ state.identity.publicKeys?.length || 0 }}</span>
          </div>
          
          <div class="identity-field">
            <label>Balance:</label>
            <span>{{ formatBalance(state.identity.balance) }} DASH</span>
          </div>
          
          <div class="identity-field">
            <label>Revision:</label>
            <span>{{ state.identity.revision }}</span>
          </div>
        </div>

        <div *ngIf="!state.identity && !state.loading && !state.error" class="not-found">
          Identity not found
        </div>
      </div>
    </div>
  `,
  styleUrls: ['./identity-display.component.scss']
})
export class IdentityDisplayComponent implements OnInit, OnDestroy {
  @Input() identityId!: string;

  private readonly wasmSDK = inject(WasmSDKAdvancedService);
  private readonly destroy$ = new Subject<void>();
  private readonly refreshTrigger$ = new BehaviorSubject<void>(undefined);

  state$: Observable<IdentityDisplayState>;

  constructor() {
    this.state$ = this.refreshTrigger$.pipe(
      switchMap(() => this.loadIdentity()),
      takeUntil(this.destroy$)
    );
  }

  ngOnInit(): void {
    this.refresh();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  refresh(): void {
    this.refreshTrigger$.next();
  }

  formatBalance(balance: number | undefined): string {
    return ((balance || 0) / 100000000).toFixed(8);
  }

  private loadIdentity(): Observable<IdentityDisplayState> {
    if (!this.identityId) {
      return of({ identity: null, loading: false, error: null });
    }

    return this.wasmSDK.getIdentityWithFallback(this.identityId).pipe(
      map(identity => ({ identity, loading: false, error: null })),
      catchError(error => of({ identity: null, loading: false, error })),
      startWith({ identity: null, loading: true, error: null }),
      finalize(() => console.log(`Identity lookup completed for ${this.identityId}`))
    );
  }
}
```

### Document List Component

```typescript
// components/document-list.component.ts
import { Component, Input, OnInit, OnDestroy, inject } from '@angular/core';
import { FormBuilder, FormGroup } from '@angular/forms';
import { Observable, Subject, BehaviorSubject, combineLatest } from 'rxjs';
import { takeUntil, switchMap, catchError, debounceTime, distinctUntilChanged } from 'rxjs/operators';
import { WasmSDKAdvancedService } from '../services/wasm-sdk-advanced.service';

interface DocumentListState {
  documents: any[];
  loading: boolean;
  error: Error | null;
  hasMore: boolean;
  currentPage: number;
}

@Component({
  selector: 'app-document-list',
  template: `
    <div class="document-list">
      <div class="header">
        <h3>Documents ({{ documentType }})</h3>
        
        <form [formGroup]="searchForm" class="search-form">
          <mat-form-field appearance="outline">
            <mat-label>Page Size</mat-label>
            <mat-select formControlName="pageSize">
              <mat-option [value]="10">10 per page</mat-option>
              <mat-option [value]="25">25 per page</mat-option>
              <mat-option [value]="50">50 per page</mat-option>
            </mat-select>
          </mat-form-field>
          
          <button 
            mat-raised-button 
            color="primary" 
            type="button"
            (click)="refresh()"
            [disabled]="(state$ | async)?.loading"
          >
            Refresh
          </button>
        </form>
      </div>

      <div *ngIf="state$ | async as state" class="content">
        <div *ngIf="state.loading" class="loading">
          <mat-progress-bar mode="indeterminate"></mat-progress-bar>
          Loading documents...
        </div>

        <div *ngIf="state.error && !state.loading" class="error">
          <mat-card class="error-card">
            <mat-card-header>
              <mat-card-title>Error Loading Documents</mat-card-title>
            </mat-card-header>
            <mat-card-content>
              <p>{{ state.error.message }}</p>
            </mat-card-content>
            <mat-card-actions>
              <button mat-raised-button color="primary" (click)="refresh()">
                Retry
              </button>
            </mat-card-actions>
          </mat-card>
        </div>

        <div *ngIf="state.documents.length === 0 && !state.loading && !state.error" class="empty">
          <mat-card>
            <mat-card-content>
              <p>No documents found</p>
            </mat-card-content>
          </mat-card>
        </div>

        <div *ngIf="state.documents.length > 0" class="documents">
          <mat-card *ngFor="let document of state.documents; trackBy: trackDocument" class="document-card">
            <mat-card-header>
              <mat-card-title>Document #{{ document.index + 1 }}</mat-card-title>
              <mat-card-subtitle class="document-id">{{ document.id }}</mat-card-subtitle>
            </mat-card-header>
            
            <mat-card-content>
              <div class="document-meta">
                <p><strong>Owner:</strong> <code>{{ document.ownerId }}</code></p>
                <p><strong>Created:</strong> {{ formatDate(document.createdAt) }}</p>
                <p><strong>Updated:</strong> {{ formatDate(document.updatedAt) }}</p>
              </div>
              
              <mat-accordion>
                <mat-expansion-panel>
                  <mat-expansion-panel-header>
                    <mat-panel-title>Document Data</mat-panel-title>
                  </mat-expansion-panel-header>
                  <pre class="document-data">{{ formatJSON(document.data) }}</pre>
                </mat-expansion-panel>
              </mat-accordion>
            </mat-card-content>
          </mat-card>
        </div>

        <div *ngIf="state.documents.length > 0" class="pagination">
          <button 
            mat-raised-button
            [disabled]="state.currentPage === 0 || state.loading"
            (click)="previousPage()"
          >
            Previous
          </button>
          
          <span class="page-info">
            Page {{ state.currentPage + 1 }}
          </span>
          
          <button 
            mat-raised-button
            [disabled]="!state.hasMore || state.loading"
            (click)="nextPage()"
          >
            Next
          </button>
        </div>
      </div>
    </div>
  `,
  styleUrls: ['./document-list.component.scss']
})
export class DocumentListComponent implements OnInit, OnDestroy {
  @Input() contractId!: string;
  @Input() documentType!: string;
  @Input() ownerId?: string;

  private readonly wasmSDK = inject(WasmSDKAdvancedService);
  private readonly fb = inject(FormBuilder);
  private readonly destroy$ = new Subject<void>();
  private readonly pageSubject$ = new BehaviorSubject<number>(0);
  
  searchForm: FormGroup;
  state$: Observable<DocumentListState>;

  constructor() {
    this.searchForm = this.fb.group({
      pageSize: [10]
    });

    const pageSize$ = this.searchForm.get('pageSize')!.valueChanges.pipe(
      startWith(this.searchForm.get('pageSize')!.value),
      debounceTime(300),
      distinctUntilChanged()
    );

    this.state$ = combineLatest([
      this.pageSubject$,
      pageSize$
    ]).pipe(
      switchMap(([page, pageSize]) => this.loadDocuments(page, pageSize)),
      takeUntil(this.destroy$)
    );
  }

  ngOnInit(): void {
    // Reset to first page when inputs change
    this.refresh();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  refresh(): void {
    this.pageSubject$.next(0);
  }

  nextPage(): void {
    this.pageSubject$.next(this.pageSubject$.value + 1);
  }

  previousPage(): void {
    this.pageSubject$.next(Math.max(0, this.pageSubject$.value - 1));
  }

  trackDocument(index: number, document: any): any {
    return document.id || index;
  }

  formatDate(timestamp: number | undefined): string {
    if (!timestamp) return 'N/A';
    return new Date(timestamp).toLocaleString();
  }

  formatJSON(data: any): string {
    return JSON.stringify(data, null, 2);
  }

  private loadDocuments(page: number, pageSize: number): Observable<DocumentListState> {
    if (!this.contractId || !this.documentType) {
      return of({
        documents: [],
        loading: false,
        error: null,
        hasMore: false,
        currentPage: page
      });
    }

    return this.wasmSDK.getDocumentsPaginated(
      this.contractId,
      this.documentType,
      page,
      pageSize
    ).pipe(
      map(({ documents, hasMore }) => ({
        documents: documents.map((doc, index) => ({ ...doc, index: page * pageSize + index })),
        loading: false,
        error: null,
        hasMore,
        currentPage: page
      })),
      catchError(error => of({
        documents: [],
        loading: false,
        error,
        hasMore: false,
        currentPage: page
      })),
      startWith({
        documents: this.pageSubject$.value === page ? [] : [], // Keep previous data while loading
        loading: true,
        error: null,
        hasMore: false,
        currentPage: page
      })
    );
  }
}
```

## RxJS Integration

### Reactive Data Service

```typescript
// services/reactive-data.service.ts
import { Injectable, inject } from '@angular/core';
import { Observable, combineLatest, timer } from 'rxjs';
import { 
  switchMap, 
  shareReplay, 
  retry, 
  startWith, 
  scan,
  distinctUntilChanged,
  filter
} from 'rxjs/operators';
import { WasmSDKAdvancedService } from './wasm-sdk-advanced.service';

@Injectable({
  providedIn: 'root'
})
export class ReactiveDataService {
  private readonly wasmSDK = inject(WasmSDKAdvancedService);

  // Auto-refreshing network status every 30 seconds
  networkStatus$ = timer(0, 30000).pipe(
    switchMap(() => this.wasmSDK.sdk$),
    switchMap(sdk => from(sdk.getNetworkStatus())),
    retry(3),
    shareReplay(1)
  );

  // Resource stats with cleanup recommendations
  resourceStats$ = timer(0, 60000).pipe(
    switchMap(() => this.wasmSDK.getResourceStats()),
    distinctUntilChanged((prev, curr) => prev.activeCount === curr.activeCount),
    shareReplay(1)
  );

  // Reactive identity cache
  private identityCache = new Map<string, Observable<any>>();

  getIdentityCached(identityId: string): Observable<any> {
    if (!this.identityCache.has(identityId)) {
      const identity$ = this.wasmSDK.getIdentityWithFallback(identityId).pipe(
        retry(2),
        shareReplay(1)
      );
      this.identityCache.set(identityId, identity$);
    }
    return this.identityCache.get(identityId)!;
  }

  // Reactive document stream with real-time updates
  getDocumentStream(contractId: string, documentType: string): Observable<any[]> {
    return timer(0, 10000).pipe( // Poll every 10 seconds
      switchMap(() => this.wasmSDK.getDocuments(contractId, documentType, { limit: 50 })),
      scan((acc: any[], curr: any[]) => {
        // Merge new documents with existing ones
        const merged = [...acc];
        curr.forEach(newDoc => {
          const existingIndex = merged.findIndex(doc => doc.id === newDoc.id);
          if (existingIndex >= 0) {
            merged[existingIndex] = newDoc; // Update existing
          } else {
            merged.push(newDoc); // Add new
          }
        });
        return merged.sort((a, b) => (b.createdAt || 0) - (a.createdAt || 0));
      }, []),
      distinctUntilChanged((prev, curr) => prev.length === curr.length),
      shareReplay(1)
    );
  }

  // Health monitoring
  sdkHealth$ = combineLatest([
    this.wasmSDK.state$,
    this.resourceStats$,
    this.networkStatus$
  ]).pipe(
    map(([sdkState, stats, network]) => ({
      sdkConnected: sdkState.connected,
      resourceCount: stats.activeCount,
      networkHeight: network.coreBlockHeight,
      memoryUsage: stats.memoryUsage,
      healthy: sdkState.connected && stats.activeCount < 1000 && network.coreBlockHeight > 0
    })),
    shareReplay(1)
  );
}
```

## State Management with NgRx

### SDK Actions

```typescript
// store/sdk.actions.ts
import { createActionGroup, emptyProps, props } from '@ngrx/store';
import { WasmSDKConfig } from '@dashevo/dash-wasm-sdk';

export const SdkActions = createActionGroup({
  source: 'SDK',
  events: {
    'Initialize': props<{ config?: Partial<WasmSDKConfig> }>(),
    'Initialize Success': emptyProps(),
    'Initialize Failure': props<{ error: any }>(),
    'Destroy': emptyProps(),
    'Destroy Success': emptyProps(),
    'Get Identity': props<{ identityId: string }>(),
    'Get Identity Success': props<{ identityId: string; identity: any }>(),
    'Get Identity Failure': props<{ identityId: string; error: any }>(),
    'Get Documents': props<{ contractId: string; documentType: string; options?: any }>(),
    'Get Documents Success': props<{ contractId: string; documentType: string; documents: any[] }>(),
    'Get Documents Failure': props<{ contractId: string; documentType: string; error: any }>()
  }
});
```

### SDK Reducer

```typescript
// store/sdk.reducer.ts
import { createReducer, on } from '@ngrx/store';
import { SdkActions } from './sdk.actions';

export interface SdkState {
  initialized: boolean;
  loading: boolean;
  error: any | null;
  identities: { [id: string]: any };
  documents: { [key: string]: any[] };
  documentLoading: { [key: string]: boolean };
}

const initialState: SdkState = {
  initialized: false,
  loading: false,
  error: null,
  identities: {},
  documents: {},
  documentLoading: {}
};

export const sdkReducer = createReducer(
  initialState,
  on(SdkActions.initialize, (state) => ({
    ...state,
    loading: true,
    error: null
  })),
  on(SdkActions.initializeSuccess, (state) => ({
    ...state,
    initialized: true,
    loading: false,
    error: null
  })),
  on(SdkActions.initializeFailure, (state, { error }) => ({
    ...state,
    initialized: false,
    loading: false,
    error
  })),
  on(SdkActions.getIdentity, (state) => ({
    ...state,
    loading: true
  })),
  on(SdkActions.getIdentitySuccess, (state, { identityId, identity }) => ({
    ...state,
    loading: false,
    identities: {
      ...state.identities,
      [identityId]: identity
    }
  })),
  on(SdkActions.getIdentityFailure, (state, { error }) => ({
    ...state,
    loading: false,
    error
  })),
  on(SdkActions.getDocuments, (state, { contractId, documentType }) => {
    const key = `${contractId}:${documentType}`;
    return {
      ...state,
      documentLoading: {
        ...state.documentLoading,
        [key]: true
      }
    };
  }),
  on(SdkActions.getDocumentsSuccess, (state, { contractId, documentType, documents }) => {
    const key = `${contractId}:${documentType}`;
    return {
      ...state,
      documents: {
        ...state.documents,
        [key]: documents
      },
      documentLoading: {
        ...state.documentLoading,
        [key]: false
      }
    };
  })
);
```

### SDK Effects

```typescript
// store/sdk.effects.ts
import { Injectable, inject } from '@angular/core';
import { Actions, createEffect, ofType } from '@ngrx/effects';
import { of } from 'rxjs';
import { map, catchError, switchMap, tap } from 'rxjs/operators';
import { WasmSDKAdvancedService } from '../services/wasm-sdk-advanced.service';
import { SdkActions } from './sdk.actions';

@Injectable()
export class SdkEffects {
  private readonly actions$ = inject(Actions);
  private readonly wasmSDK = inject(WasmSDKAdvancedService);

  initialize$ = createEffect(() =>
    this.actions$.pipe(
      ofType(SdkActions.initialize),
      switchMap(({ config }) =>
        this.wasmSDK.initializeWithRetry(config).pipe(
          map(() => SdkActions.initializeSuccess()),
          catchError(error => of(SdkActions.initializeFailure({ error })))
        )
      )
    )
  );

  getIdentity$ = createEffect(() =>
    this.actions$.pipe(
      ofType(SdkActions.getIdentity),
      switchMap(({ identityId }) =>
        this.wasmSDK.getIdentityWithFallback(identityId).pipe(
          map(identity => SdkActions.getIdentitySuccess({ identityId, identity })),
          catchError(error => of(SdkActions.getIdentityFailure({ identityId, error })))
        )
      )
    )
  );

  getDocuments$ = createEffect(() =>
    this.actions$.pipe(
      ofType(SdkActions.getDocuments),
      switchMap(({ contractId, documentType, options }) =>
        this.wasmSDK.getDocuments(contractId, documentType, options || {}).pipe(
          map(documents => SdkActions.getDocumentsSuccess({ contractId, documentType, documents })),
          catchError(error => of(SdkActions.getDocumentsFailure({ contractId, documentType, error })))
        )
      )
    )
  );
}
```

## Standalone Components

### Modern Standalone SDK Component

```typescript
// components/standalone-sdk.component.ts
import { Component, OnInit, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ReactiveFormsModule, FormBuilder, FormGroup, Validators } from '@angular/forms';
import { MatCardModule } from '@angular/material/card';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { MatButtonModule } from '@angular/material/button';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { Observable } from 'rxjs';
import { WasmSDKAdvancedService } from '../services/wasm-sdk-advanced.service';

@Component({
  selector: 'app-standalone-sdk',
  standalone: true,
  imports: [
    CommonModule,
    ReactiveFormsModule,
    MatCardModule,
    MatFormFieldModule,
    MatInputModule,
    MatButtonModule,
    MatProgressSpinnerModule
  ],
  providers: [
    WasmSDKAdvancedService
  ],
  template: `
    <mat-card>
      <mat-card-header>
        <mat-card-title>Standalone SDK Component</mat-card-title>
        <mat-card-subtitle>
          Status: {{ (sdkService.state$ | async)?.connected ? 'Connected' : 'Disconnected' }}
        </mat-card-subtitle>
      </mat-card-header>

      <mat-card-content>
        <form [formGroup]="searchForm" (ngSubmit)="onSubmit()">
          <mat-form-field appearance="outline" class="full-width">
            <mat-label>Identity ID</mat-label>
            <input matInput formControlName="identityId" placeholder="Enter identity ID...">
            <mat-error *ngIf="searchForm.get('identityId')?.hasError('required')">
              Identity ID is required
            </mat-error>
          </mat-form-field>

          <button 
            mat-raised-button 
            color="primary" 
            type="submit"
            [disabled]="searchForm.invalid || (loading$ | async)"
            class="search-button"
          >
            <mat-spinner *ngIf="loading$ | async" diameter="20"></mat-spinner>
            {{ (loading$ | async) ? 'Searching...' : 'Search Identity' }}
          </button>
        </form>

        <div *ngIf="result$ | async as result" class="result">
          <h3>Search Result</h3>
          <pre>{{ result | json }}</pre>
        </div>

        <div *ngIf="error$ | async as error" class="error">
          <h3>Error</h3>
          <p>{{ error.message }}</p>
        </div>
      </mat-card-content>
    </mat-card>
  `,
  styles: [`
    .full-width {
      width: 100%;
      margin-bottom: 1rem;
    }
    
    .search-button {
      width: 100%;
      margin-bottom: 1rem;
    }
    
    .result, .error {
      margin-top: 1rem;
      padding: 1rem;
      border-radius: 4px;
    }
    
    .result {
      background-color: #e8f5e8;
      border: 1px solid #4caf50;
    }
    
    .error {
      background-color: #ffebee;
      border: 1px solid #f44336;
      color: #d32f2f;
    }
    
    pre {
      font-size: 0.8em;
      overflow-x: auto;
      white-space: pre-wrap;
    }
  `]
})
export class StandaloneSdkComponent implements OnInit {
  readonly sdkService = inject(WasmSDKAdvancedService);
  private readonly fb = inject(FormBuilder);

  searchForm: FormGroup;
  result$: Observable<any> | null = null;
  error$: Observable<any> | null = null;
  loading$: Observable<boolean>;

  constructor() {
    this.searchForm = this.fb.group({
      identityId: ['', [Validators.required, Validators.minLength(10)]]
    });

    this.loading$ = this.sdkService.state$.pipe(
      map(state => state.loading)
    );
  }

  ngOnInit(): void {
    // Initialize SDK when component loads
    this.sdkService.initializeWithRetry().subscribe();
  }

  onSubmit(): void {
    if (this.searchForm.valid) {
      const identityId = this.searchForm.value.identityId;
      
      this.result$ = this.sdkService.getIdentityWithFallback(identityId).pipe(
        tap(identity => {
          this.error$ = null;
          console.log('Identity found:', identity);
        }),
        catchError(error => {
          this.error$ = of(error);
          this.result$ = null;
          return EMPTY;
        })
      );
    }
  }
}
```

## Complete Dashboard Example

```typescript
// components/dashboard.component.ts
import { Component, OnInit, OnDestroy, inject } from '@angular/core';
import { FormBuilder, FormGroup } from '@angular/forms';
import { Observable, Subject, BehaviorSubject, combineLatest } from 'rxjs';
import { takeUntil, startWith, distinctUntilChanged, debounceTime } from 'rxjs/operators';
import { WasmSDKAdvancedService } from '../services/wasm-sdk-advanced.service';
import { ReactiveDataService } from '../services/reactive-data.service';

interface DashboardState {
  activeTab: string;
  identityId: string;
  contractId: string;
  documentType: string;
}

@Component({
  selector: 'app-dashboard',
  template: `
    <div class="dashboard">
      <header class="dashboard-header">
        <h1>Dash Platform Dashboard</h1>
        <app-sdk-status></app-sdk-status>
      </header>

      <div class="dashboard-content">
        <aside class="sidebar">
          <nav class="nav-menu">
            <button 
              *ngFor="let tab of tabs" 
              [class.active]="(dashboardState$ | async)?.activeTab === tab.key"
              (click)="setActiveTab(tab.key)"
            >
              <mat-icon>{{ tab.icon }}</mat-icon>
              {{ tab.label }}
            </button>
          </nav>

          <div class="settings">
            <h3>Settings</h3>
            <form [formGroup]="settingsForm">
              <mat-form-field appearance="outline">
                <mat-label>Network</mat-label>
                <mat-select formControlName="network">
                  <mat-option value="testnet">Testnet</mat-option>
                  <mat-option value="mainnet">Mainnet</mat-option>
                </mat-select>
              </mat-form-field>

              <mat-form-field appearance="outline">
                <mat-label>Identity ID</mat-label>
                <input matInput formControlName="identityId" placeholder="Enter identity ID...">
              </mat-form-field>

              <mat-form-field appearance="outline">
                <mat-label>Contract ID</mat-label>
                <input matInput formControlName="contractId" placeholder="Enter contract ID...">
              </mat-form-field>
            </form>
          </div>

          <div class="system-stats" *ngIf="healthInfo$ | async as health">
            <h3>System Health</h3>
            <div class="stat-item" [class.healthy]="health.healthy" [class.unhealthy]="!health.healthy">
              <mat-icon>{{ health.healthy ? 'check_circle' : 'error' }}</mat-icon>
              <span>{{ health.healthy ? 'Healthy' : 'Issues Detected' }}</span>
            </div>
            <div class="stat-item">
              <span>Resources: {{ health.resourceCount }}</span>
            </div>
            <div class="stat-item">
              <span>Block Height: {{ health.networkHeight }}</span>
            </div>
          </div>
        </aside>

        <main class="main-content">
          <div [ngSwitch]="(dashboardState$ | async)?.activeTab">
            <app-identity-display 
              *ngSwitchCase="'identity'"
              [identityId]="(dashboardState$ | async)?.identityId"
            ></app-identity-display>

            <app-document-list 
              *ngSwitchCase="'documents'"
              [contractId]="(dashboardState$ | async)?.contractId"
              [documentType]="(dashboardState$ | async)?.documentType"
              [ownerId]="(dashboardState$ | async)?.identityId"
            ></app-document-list>

            <app-network-status 
              *ngSwitchCase="'network'"
            ></app-network-status>

            <div *ngSwitchDefault class="welcome">
              <h2>Welcome to Dash Platform Dashboard</h2>
              <p>Select a tab to get started</p>
            </div>
          </div>
        </main>
      </div>
    </div>
  `,
  styleUrls: ['./dashboard.component.scss']
})
export class DashboardComponent implements OnInit, OnDestroy {
  private readonly wasmSDK = inject(WasmSDKAdvancedService);
  private readonly reactiveData = inject(ReactiveDataService);
  private readonly fb = inject(FormBuilder);
  private readonly destroy$ = new Subject<void>();

  tabs = [
    { key: 'identity', label: 'Identity', icon: 'person' },
    { key: 'documents', label: 'Documents', icon: 'description' },
    { key: 'network', label: 'Network', icon: 'cloud' }
  ];

  settingsForm: FormGroup;
  dashboardState$: Observable<DashboardState>;
  healthInfo$: Observable<any>;

  private readonly stateSubject$ = new BehaviorSubject<Partial<DashboardState>>({
    activeTab: 'identity'
  });

  constructor() {
    this.settingsForm = this.fb.group({
      network: ['testnet'],
      identityId: [''],
      contractId: [''],
      documentType: ['note']
    });

    // Combine form values with internal state
    this.dashboardState$ = combineLatest([
      this.stateSubject$,
      this.settingsForm.valueChanges.pipe(
        startWith(this.settingsForm.value),
        debounceTime(300),
        distinctUntilChanged()
      )
    ]).pipe(
      map(([internalState, formValues]) => ({
        activeTab: internalState.activeTab || 'identity',
        identityId: formValues.identityId,
        contractId: formValues.contractId,
        documentType: formValues.documentType
      })),
      takeUntil(this.destroy$)
    );

    this.healthInfo$ = this.reactiveData.sdkHealth$;
  }

  ngOnInit(): void {
    // Initialize SDK
    this.wasmSDK.initializeWithRetry().subscribe();

    // Handle network changes
    this.settingsForm.get('network')?.valueChanges.pipe(
      distinctUntilChanged(),
      takeUntil(this.destroy$)
    ).subscribe(network => {
      this.wasmSDK.destroy().then(() => {
        this.wasmSDK.initialize({ network });
      });
    });
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  setActiveTab(tab: string): void {
    this.stateSubject$.next({ activeTab: tab });
  }
}
```

## TypeScript Integration

### Type Safety Configuration

```typescript
// types/sdk-types.ts
import { WasmSDK, WasmSDKConfig } from '@dashevo/dash-wasm-sdk';

export interface TypedIdentity {
  id: string;
  publicKeys: PublicKey[];
  balance: number;
  revision: number;
  createdAt?: number;
  updatedAt?: number;
}

export interface PublicKey {
  id: number;
  type: number;
  purpose: number;
  securityLevel: number;
  data: Uint8Array;
  readOnly: boolean;
}

export interface TypedDocument {
  id: string;
  ownerId: string;
  dataContractId: string;
  revision: number;
  data: Record<string, any>;
  createdAt?: number;
  updatedAt?: number;
}

export interface DataContract {
  id: string;
  ownerId: string;
  version: number;
  documents: Record<string, DocumentSchema>;
}

export interface DocumentSchema {
  type: string;
  properties: Record<string, PropertySchema>;
  additionalProperties: boolean;
  required?: string[];
}

export interface PropertySchema {
  type: string;
  maxLength?: number;
  minLength?: number;
  minimum?: number;
  maximum?: number;
  format?: string;
}

// Typed service interface
export interface ITypedWasmSDKService {
  getIdentity(identityId: string): Observable<TypedIdentity | null>;
  getDocuments<T = any>(contractId: string, documentType: string, options?: any): Observable<TypedDocument<T>[]>;
  getDataContract(contractId: string): Observable<DataContract | null>;
}
```

## Testing

### Service Testing

```typescript
// services/wasm-sdk.service.spec.ts
import { TestBed } from '@angular/core/testing';
import { of, throwError } from 'rxjs';
import { WasmSDKAdvancedService } from './wasm-sdk-advanced.service';
import { WasmSDK } from '@dashevo/dash-wasm-sdk';

describe('WasmSDKAdvancedService', () => {
  let service: WasmSDKAdvancedService;
  let mockWasmSDK: jasmine.SpyObj<WasmSDK>;

  beforeEach(() => {
    const spy = jasmine.createSpyObj('WasmSDK', ['initialize', 'destroy', 'getIdentity', 'getDocuments']);
    
    TestBed.configureTestingModule({
      providers: [
        WasmSDKAdvancedService,
        { provide: WasmSDK, useValue: spy }
      ]
    });
    
    service = TestBed.inject(WasmSDKAdvancedService);
    mockWasmSDK = TestBed.inject(WasmSDK) as jasmine.SpyObj<WasmSDK>;
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should initialize SDK successfully', async () => {
    mockWasmSDK.initialize.and.returnValue(Promise.resolve());
    
    await service.initialize();
    
    expect(mockWasmSDK.initialize).toHaveBeenCalled();
    service.state$.subscribe(state => {
      expect(state.initialized).toBe(true);
      expect(state.connected).toBe(true);
      expect(state.loading).toBe(false);
    });
  });

  it('should handle initialization errors', async () => {
    const error = new Error('Initialization failed');
    mockWasmSDK.initialize.and.returnValue(Promise.reject(error));
    
    try {
      await service.initialize();
      fail('Expected error to be thrown');
    } catch (e) {
      expect(e).toBe(error);
    }
    
    service.state$.subscribe(state => {
      expect(state.initialized).toBe(false);
      expect(state.connected).toBe(false);
      expect(state.error).toBe(error);
    });
  });

  it('should get identity successfully', () => {
    const mockIdentity = { id: 'test-id', balance: 1000 };
    mockWasmSDK.getIdentity.and.returnValue(Promise.resolve(mockIdentity));
    
    // Mock the SDK being ready
    spyOn(service, 'sdk$').and.returnValue(of(mockWasmSDK));
    
    service.getIdentity('test-id').subscribe(identity => {
      expect(identity).toEqual(mockIdentity);
    });
  });
});
```

### Component Testing

```typescript
// components/identity-display.component.spec.ts
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { of, throwError } from 'rxjs';
import { IdentityDisplayComponent } from './identity-display.component';
import { WasmSDKAdvancedService } from '../services/wasm-sdk-advanced.service';

describe('IdentityDisplayComponent', () => {
  let component: IdentityDisplayComponent;
  let fixture: ComponentFixture<IdentityDisplayComponent>;
  let mockSDKService: jasmine.SpyObj<WasmSDKAdvancedService>;

  beforeEach(async () => {
    const spy = jasmine.createSpyObj('WasmSDKAdvancedService', ['getIdentityWithFallback']);
    
    await TestBed.configureTestingModule({
      declarations: [IdentityDisplayComponent],
      providers: [
        { provide: WasmSDKAdvancedService, useValue: spy }
      ]
    }).compileComponents();
    
    fixture = TestBed.createComponent(IdentityDisplayComponent);
    component = fixture.componentInstance;
    mockSDKService = TestBed.inject(WasmSDKAdvancedService) as jasmine.SpyObj<WasmSDKAdvancedService>;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should display identity when loaded successfully', () => {
    const mockIdentity = {
      id: 'test-identity',
      balance: 1000000000,
      publicKeys: [{ id: 0 }]
    };
    
    mockSDKService.getIdentityWithFallback.and.returnValue(of(mockIdentity));
    component.identityId = 'test-identity';
    
    fixture.detectChanges();
    
    component.state$.subscribe(state => {
      expect(state.identity).toEqual(mockIdentity);
      expect(state.loading).toBe(false);
      expect(state.error).toBeNull();
    });
  });

  it('should handle identity loading errors', () => {
    const error = new Error('Identity not found');
    mockSDKService.getIdentityWithFallback.and.returnValue(throwError(() => error));
    component.identityId = 'invalid-identity';
    
    fixture.detectChanges();
    
    component.state$.subscribe(state => {
      expect(state.identity).toBeNull();
      expect(state.loading).toBe(false);
      expect(state.error).toBe(error);
    });
  });
});
```

## Troubleshooting

### Common Angular-Specific Issues

#### 1. Zone.js Compatibility

```typescript
// If you encounter Zone.js issues with WASM operations
import { NgZone } from '@angular/core';

@Injectable()
export class WasmSDKService {
  constructor(private ngZone: NgZone) {}
  
  async initialize(): Promise<void> {
    // Run WASM operations outside Angular zone
    return this.ngZone.runOutsideAngular(async () => {
      this.sdkInstance = new WasmSDK(config);
      await this.sdkInstance.initialize();
      
      // Trigger change detection when done
      this.ngZone.run(() => {
        this.updateState({ initialized: true });
      });
    });
  }
}
```

#### 2. Memory Management in Services

```typescript
// Proper cleanup in services
@Injectable({
  providedIn: 'root'
})
export class WasmSDKService implements OnDestroy {
  private cleanupInterval?: number;
  
  ngOnDestroy(): void {
    if (this.cleanupInterval) {
      clearInterval(this.cleanupInterval);
    }
    this.destroy();
  }
  
  private startResourceCleanup(): void {
    this.cleanupInterval = window.setInterval(() => {
      if (this.sdkInstance) {
        this.sdkInstance.cleanupResources();
      }
    }, 300000); // 5 minutes
  }
}
```

#### 3. Change Detection Optimization

```typescript
// Use OnPush change detection for better performance
@Component({
  selector: 'app-identity-display',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `...`
})
export class IdentityDisplayComponent {
  constructor(private cdr: ChangeDetectorRef) {}
  
  private updateView(): void {
    // Manually trigger change detection when needed
    this.cdr.markForCheck();
  }
}
```

#### 4. Lazy Loading Module Issues

```typescript
// For lazy-loaded modules, provide SDK in the feature module
@NgModule({
  imports: [
    CommonModule,
    WasmSDKModule.forFeature() // Use forFeature() in lazy modules
  ]
})
export class FeatureModule {}
```

This Angular integration provides a production-ready foundation for building enterprise Dash Platform applications with proper dependency injection, reactive programming patterns, state management, and comprehensive testing support.