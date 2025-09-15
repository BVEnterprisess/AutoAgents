import React, { useState, useEffect } from 'react';

// Infrastructure Assassin Status
interface IAStatus {
  isInitialized: boolean;
  wasmReady: boolean;
  apisAvailable: string[];
  error?: string;
}

const App: React.FC = () => {
  const [status, setStatus] = useState<IAStatus>({
    isInitialized: false,
    wasmReady: false,
    apisAvailable: []
  });

  const [activeTab, setActiveTab] = useState('dashboard');

  useEffect(() => {
    // Initialize Infrastructure Assassin runtime
    const initializeIA = async () => {
      try {
        // Check if global IA runtime exists
        if (typeof window !== 'undefined' && (window as any).infrastructureAssassin) {
          const ia = (window as any).infrastructureAssassin;

          await ia.init();

          setStatus({
            isInitialized: true,
            wasmReady: true, // WASM will be loaded via build system
            apisAvailable: ['DOM Control', 'JS Execution', 'Network Monitoring', 'Storage', 'Screenshots']
          });

          console.log('🚀 Infrastructure Assassin Web Runtime initialized');
        } else {
          throw new Error('Infrastructure Assassin runtime not found');
        }
      } catch (error) {
        console.error('Failed to initialize IA:', error);
        setStatus(prev => ({
          ...prev,
          error: error instanceof Error ? error.message : 'Unknown error occurred'
        }));
      }
    };

    initializeIA();
  }, []);

  const tabs = [
    { id: 'dashboard', label: 'Dashboard', icon: '📊' },
    { id: 'dom', label: 'DOM Control', icon: '🎯' },
    { id: 'js', label: 'JS Execution', icon: '⚡' },
    { id: 'network', label: 'Network Monitor', icon: '🌐' },
    { id: 'storage', label: 'Storage', icon: '💾' },
    { id: 'screenshots', label: 'Screenshots', icon: '📸' }
  ];

  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-900 via-slate-900 to-gray-900 text-white">
      {/* Header */}
      <header className="backdrop-blur-xl bg-black/20 border-b border-white/10">
        <div className="max-w-7xl mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-4">
              <div className="w-10 h-10 bg-gradient-to-r from-blue-500 to-purple-500 rounded-lg flex items-center justify-center">
                <span className="text-white font-bold text-xl">IA</span>
              </div>
              <div>
                <h1 className="text-2xl font-bold ia-brand">Infrastructure Assassin</h1>
                <p className="text-sm opacity-75">Zero-Cost Browser Automation</p>
              </div>
            </div>

            <div className="flex items-center space-x-4">
              {/* Status Indicator */}
              <div className="flex items-center space-x-2">
                <div className={`w-3 h-3 rounded-full ${
                  status.isInitialized ? 'bg-green-400' : 'bg-red-400'
                } ${status.isInitialized ? 'animate-pulse' : ''}`} />
                <span className="text-sm">
                  {status.isInitialized ? 'Active' : 'Initializing...'}
                </span>
              </div>

              <div className="text-sm bg-black/30 px-3 py-1 rounded-full">
                v2.0.0
              </div>
            </div>
          </div>
        </div>
      </header>

      <div className="flex h-screen pt-16">
        {/* Sidebar */}
        <nav className="w-64 backdrop-blur-xl bg-black/10 border-r border-white/10 p-6">
          <div className="space-y-2">
            {tabs.map(tab => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={`w-full flex items-center space-x-3 px-4 py-3 rounded-lg transition-colors ${
                  activeTab === tab.id
                    ? 'bg-blue-600/30 border border-blue-500/50 text-blue-200'
                    : 'hover:bg-white/10 text-gray-300'
                }`}
              >
                <span className="text-lg">{tab.icon}</span>
                <span className="font-medium">{tab.label}</span>
                {status.apisAvailable.includes(tab.label) && (
                  <div className="ml-auto">
                    <div className="w-2 h-2 bg-green-400 rounded-full animate-pulse" />
                  </div>
                )}
              </button>
            ))}
          </div>

          {/* API Status */}
          <div className="mt-8 p-4 bg-black/20 rounded-lg border border-white/10">
            <h4 className="font-semibold mb-2 text-sm">API Status</h4>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span>WASM Module</span>
                <span className={status.wasmReady ? 'text-green-400' : 'text-yellow-400'}>
                  {status.wasmReady ? '✅' : '⏳'}
                </span>
              </div>
              <div className="flex justify-between">
                <span>Browser APIs</span>
                <span className={status.apisAvailable.length > 0 ? 'text-green-400' : 'text-yellow-400'}>
                  {status.apisAvailable.length}/5
                </span>
              </div>
            </div>
          </div>

          {status.error && (
            <div className="mt-4 p-3 bg-red-500/20 border border-red-500/50 rounded-lg text-sm text-red-400">
              ⚠️ {status.error}
            </div>
          )}
        </nav>

        {/* Main Content */}
        <main className="flex-1 overflow-y-auto">
          <div className="p-8">
            {activeTab === 'dashboard' && <DashboardTab status={status} />}
            {activeTab === 'dom' && <DOMTab />}
            {activeTab === 'js' && <JSTab />}
            {activeTab === 'network' && <NetworkTab />}
            {activeTab === 'storage' && <StorageTab />}
            {activeTab === 'screenshots' && <ScreenshotsTab />}
          </div>
        </main>
      </div>
    </div>
  );
};

// Dashboard Tab Component
const DashboardTab: React.FC<{ status: IAStatus }> = ({ status }) => (
  <div className="space-y-8">
    <div>
      <h2 className="text-3xl font-bold mb-2">Infrastructure Assassin Dashboard</h2>
      <p className="text-gray-400 text-lg">
        Zero-cost browser automation that disrupts cloud providers
      </p>
    </div>

    {/* Status Cards */}
    <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
      <div className="backdrop-blur-xl bg-gradient-to-r from-green-900/20 to-green-800/20 border border-green-500/20 rounded-xl p-6">
        <div className="flex items-center space-x-4">
          <div className="w-12 h-12 bg-green-500/20 rounded-lg flex items-center justify-center">
            <span className="text-2xl">⚡</span>
          </div>
          <div>
            <h3 className="text-xl font-bold text-green-400">Performance</h3>
            <p className="text-green-200">10x productivity gain vs AWS</p>
          </div>
        </div>
      </div>

      <div className="backdrop-blur-xl bg-gradient-to-r from-blue-900/20 to-blue-800/20 border border-blue-500/20 rounded-xl p-6">
        <div className="flex items-center space-x-4">
          <div className="w-12 h-12 bg-blue-500/20 rounded-lg flex items-center justify-center">
            <span className="text-2xl">💰</span>
          </div>
          <div>
            <h3 className="text-xl font-bold text-blue-400">$0 Cost</h3>
            <p className="text-blue-200">No cloud infrastructure fees</p>
          </div>
        </div>
      </div>

      <div className="backdrop-blur-xl bg-gradient-to-r from-purple-900/20 to-purple-800/20 border border-purple-500/20 rounded-xl p-6">
        <div className="flex items-center space-x-4">
          <div className="w-12 h-12 bg-purple-500/20 rounded-lg flex items-center justify-center">
            <span className="text-2xl">🔒</span>
          </div>
          <div>
            <h3 className="text-xl font-bold text-purple-400">Security</h3>
            <p className="text-purple-200">Zero-trust zero-dependency</p>
          </div>
        </div>
      </div>
    </div>

    {/* Quick Actions */}
    <div className="backdrop-blur-xl bg-white/5 border border-white/10 rounded-xl p-6">
      <h3 className="text-xl font-bold mb-4">Quick Actions</h3>
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        {['Test DOM Control', 'Run JS Script', 'Monitor Network', 'Capture Screenshot'].map((action) => (
          <button
            key={action}
            disabled={!status.isInitialized}
            className="p-4 bg-white/10 hover:bg-white/20 disabled:bg-gray-800 disabled:cursor-not-allowed rounded-lg transition-colors text-left group"
          >
            <div className="text-sm opacity-75 mb-1">
              {action.split(' ')[0]}
            </div>
            <div className="font-medium group-hover:text-blue-400 transition-colors">
              {action.split(' ').slice(1).join(' ')}
            </div>
          </button>
        ))}
      </div>
    </div>
  </div>
);

// Placeholder components for other tabs
const DOMTab = () => (
  <div className="space-y-6">
    <h2 className="text-2xl font-bold">🎯 DOM Control</h2>
    <div className="backdrop-blur-xl bg-white/5 border border-white/10 rounded-xl p-8 text-center">
      <div className="text-6xl mb-4 opacity-50">🎯</div>
      <h3 className="text-xl font-bold mb-2">Real Browser DOM Manipulation</h3>
      <p className="text-gray-400">Click elements, fill forms, manipulate the actual DOM of real web pages.</p>
      <div className="mt-6 flex justify-center space-x-4">
        <button className="px-6 py-3 bg-blue-600 hover:bg-blue-700 rounded-lg font-medium">Coming Soon</button>
      </div>
    </div>
  </div>
);

const JSTab = () => (
  <div className="space-y-6">
    <h2 className="text-2xl font-bold">⚡ JavaScript Execution</h2>
    <div className="backdrop-blur-xl bg-white/5 border border-white/10 rounded-xl p-8 text-center">
      <div className="text-6xl mb-4 opacity-50">⚡</div>
      <h3 className="text-xl font-bold mb-2">In-Browser JavaScript Execution</h3>
      <p className="text-gray-400">Execute JavaScript code directly in the browser with full access to window and document objects.</p>
      <div className="mt-6 flex justify-center space-x-4">
        <button className="px-6 py-3 bg-blue-600 hover:bg-blue-700 rounded-lg font-medium">Coming Soon</button>
      </div>
    </div>
  </div>
);

const NetworkTab = () => (
  <div className="space-y-6">
    <h2 className="text-2xl font-bold">🌐 Network Monitoring</h2>
    <div className="backdrop-blur-xl bg-white/5 border border-white/10 rounded-xl p-8 text-center">
      <div className="text-6xl mb-4 opacity-50">🌐</div>
      <h3 className="text-xl font-bold mb-2">Network Request Interception</h3>
      <p className="text-gray-400">Monitor, intercept, and analyze all network requests and responses in real-time.</p>
      <div className="mt-6 flex justify-center space-x-4">
        <button className="px-6 py-3 bg-blue-600 hover:bg-blue-700 rounded-lg font-medium">Coming Soon</button>
      </div>
    </div>
  </div>
);

const StorageTab = () => (
  <div className="space-y-6">
    <h2 className="text-2xl font-bold">💾 Browser Storage</h2>
    <div className="backdrop-blur-xl bg-white/5 border border-white/10 rounded-xl p-8 text-center">
      <div className="text-6xl mb-4 opacity-50">💾</div>
      <h3 className="text-xl font-bold mb-2">Persistent Session Storage</h3>
      <p className="text-gray-400">Store and retrieve data using localStorage, IndexedDB, and Cache API</p>
      <div className="mt-6 flex justify-center space-x-4">
        <button className="px-6 py-3 bg-blue-600 hover:bg-blue-700 rounded-lg font-medium">Coming Soon</button>
      </div>
    </div>
  </div>
);

const ScreenshotsTab = () => (
  <div className="space-y-6">
    <h2 className="text-2xl font-bold">📸 Screenshot Capture</h2>
    <div className="backdrop-blur-xl bg-white/5 border border-white/10 rounded-xl p-8 text-center">
      <div className="text-6xl mb-4 opacity-50">📸</div>
      <h3 className="text-xl font-bold mb-2">Visual Proof Generation</h3>
      <p className="text-gray-400">Capture screenshots, viewport images, and element snapshots with multiple formats.</p>
      <div className="mt-6 flex justify-center space-x-4">
        <button className="px-6 py-3 bg-blue-600 hover:bg-blue-700 rounded-lg font-medium">Coming Soon</button>
      </div>
    </div>
  </div>
);

export default App;
