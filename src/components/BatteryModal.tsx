import { memo } from "react";
import { X } from "lucide-react";
import type { BatterySummaryResult } from "../api";
import {
  formatBatteryCapacity,
  formatBatteryMinutes,
  formatBatteryPercent,
  getBatteryWearLevel,
} from "../batteryUtils";

type BatteryModalProps = {
  open: boolean;
  loading: boolean;
  summary: BatterySummaryResult | null;
  error: string;
  onRefresh: () => void;
  onClose: () => void;
};

export const BatteryModal = memo(function BatteryModal({
  open,
  loading,
  summary,
  error,
  onRefresh,
  onClose,
}: BatteryModalProps) {
  if (!open) {
    return null;
  }

  return (
    <div className="fixed inset-0 z-50 bg-slate-950/60 flex items-center justify-center px-4">
      <div className="battery-modal">
        <div className="battery-modal-header">
          <div>
            <h3 className="text-base font-bold text-slate-100">Battery Info</h3>
            <p className="text-xs text-slate-400 mt-0.5">
              Summary focused on wear level and estimated battery lifetime.
            </p>
          </div>
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={onRefresh}
              disabled={loading}
              className="capsule-btn compact-pill battery-refresh-btn"
            >
              {loading ? "Loading..." : "Refresh"}
            </button>
            <button
              type="button"
              onClick={onClose}
              disabled={loading}
              className="battery-close-btn capsule-btn"
              title="Close"
            >
              <X className="w-4 h-4" />
            </button>
          </div>
        </div>

        <div className="battery-modal-body">
          {loading ? (
            <div className="battery-placeholder">Loading battery summary...</div>
          ) : error ? (
            <div className="battery-placeholder battery-placeholder-error">
              Unable to load battery summary: {error}
            </div>
          ) : summary ? (
            <div className="battery-summary-shell">
              {!summary.present ? (
                <div className="battery-placeholder">
                  {summary.note || "No battery detected on this machine."}
                </div>
              ) : (
                <>
                  <div className="battery-summary-primary-grid">
                      <div className="battery-summary-card battery-summary-card-health">
                        <div className="battery-summary-label">Health Remaining</div>
                        <div className="battery-summary-value">
                          {formatBatteryPercent(summary.health_percent)}
                        </div>
                        <div className="battery-summary-hint">
                          Full charge / design capacity
                        </div>
                      </div>
                    <div className="battery-summary-card battery-summary-card-wear">
                      <div className="battery-summary-label">Wear Level</div>
                      <div className="battery-summary-value">
                        {formatBatteryPercent(summary.wear_percent)}
                      </div>
                      <div className="battery-summary-hint">
                        {getBatteryWearLevel(summary.wear_percent)}
                      </div>
                    </div>
                  </div>

                  <div className="battery-summary-grid">
                    <div className="battery-stat">
                      <span className="battery-stat-title">Current Charge</span>
                      <span className="battery-stat-value">
                        {formatBatteryPercent(summary.charge_percent, 0)}
                      </span>
                    </div>
                    <div className="battery-stat">
                      <span className="battery-stat-title">Remaining Runtime</span>
                      <span className="battery-stat-value">
                        {formatBatteryMinutes(summary.estimated_runtime_minutes)}
                      </span>
                    </div>
                    <div className="battery-stat">
                      <span className="battery-stat-title">Runtime At Full (est.)</span>
                      <span className="battery-stat-value">
                        {formatBatteryMinutes(summary.estimated_runtime_full_minutes)}
                      </span>
                    </div>
                    <div className="battery-stat">
                      <span className="battery-stat-title">Cycle Count</span>
                      <span className="battery-stat-value">
                        {summary.cycle_count === null || summary.cycle_count === undefined ? "--" : summary.cycle_count}
                      </span>
                    </div>
                    <div className="battery-stat">
                      <span className="battery-stat-title">Design Capacity</span>
                      <span className="battery-stat-value">
                        {formatBatteryCapacity(summary.design_capacity_mwh)}
                      </span>
                    </div>
                    <div className="battery-stat">
                      <span className="battery-stat-title">Full Charge Capacity</span>
                      <span className="battery-stat-value">
                        {formatBatteryCapacity(summary.full_charge_capacity_mwh)}
                      </span>
                    </div>
                  </div>

                  <div className="battery-summary-status-row">
                    <span className="battery-status-chip">{summary.status}</span>
                    <span className="battery-summary-note">{summary.note}</span>
                  </div>
                </>
              )}
            </div>
          ) : (
            <div className="battery-placeholder">No battery summary available.</div>
          )}
        </div>
      </div>
    </div>
  );
});
