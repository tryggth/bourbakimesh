import React from 'react';
import { RefreshCw, CheckCircle, WifiOff } from 'lucide-react';

interface UpdateNotificationProps {
  updateAvailable: boolean;
  onRefresh: () => void;
  onDismiss?: () => void;
}

export const UpdateNotification: React.FC<UpdateNotificationProps> = ({
  updateAvailable,
  onRefresh,
  onDismiss,
}) => {
  const isOnline = typeof navigator !== 'undefined' ? navigator.onLine : true;
  const version = typeof __APP_VERSION__ !== 'undefined' ? __APP_VERSION__ : 'dev';
  const buildTime = typeof __APP_BUILD_TIME__ !== 'undefined' ? new Date(__APP_BUILD_TIME__).toLocaleString() : 'Live Dev';

  return (
    <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 font-sans max-w-md">
      {/* Offline Indicator if disconnected */}
      {!isOnline && (
        <div className="flex items-center gap-2 px-3 py-2 bg-amber-950/90 border border-amber-800/80 rounded-lg text-amber-300 text-xs shadow-lg backdrop-blur">
          <WifiOff className="w-4 h-4 text-amber-400 shrink-0" />
          <span>Offline mode active. Using cached IndexedDB proof DAG.</span>
        </div>
      )}

      {/* Auto-Update Banner */}
      {updateAvailable && (
        <div className="flex items-center justify-between gap-3 p-4 bg-blue-950/95 border border-blue-600 rounded-xl shadow-2xl backdrop-blur text-white animate-in slide-in-from-bottom duration-300">
          <div className="flex items-center gap-3 min-w-0">
            <div className="p-2 bg-blue-600 rounded-lg text-white">
              <RefreshCw className="w-5 h-5 animate-spin" />
            </div>
            <div>
              <h4 className="text-xs font-bold uppercase tracking-wider text-blue-200">
                New Version Available
              </h4>
              <p className="text-xs text-slate-300">
                PWA cache refreshed. Reload to update to latest build.
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2 shrink-0">
            <button
              onClick={onRefresh}
              className="px-3 py-1.5 bg-blue-600 hover:bg-blue-500 text-white rounded-lg text-xs font-semibold shadow transition-colors flex items-center gap-1.5"
            >
              <CheckCircle className="w-3.5 h-3.5" />
              Reload
            </button>
            {onDismiss && (
              <button
                onClick={onDismiss}
                className="px-2 py-1.5 text-slate-400 hover:text-white text-xs transition-colors"
              >
                Later
              </button>
            )}
          </div>
        </div>
      )}

      {/* Build Info Pill */}
      <div className="self-end px-2.5 py-1 bg-slate-950/70 border border-slate-800/80 rounded-full text-[10px] text-slate-400 font-mono flex items-center gap-2 backdrop-blur shadow">
        <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
        <span>v{version}</span>
        <span>&bull;</span>
        <span>{buildTime}</span>
      </div>
    </div>
  );
};
