import { Flame, Globe, Plus, Trash2 } from "lucide-react";

import type { NetworkInterface } from "../api";
import type { RouteValidationErrors, RouteValidationInput } from "../networkValidation";
import { ActionBtn, Field } from "./AppChrome";

interface RouteConfigPanelProps {
  routeForm: RouteValidationInput;
  visibleRouteFormErrors: RouteValidationErrors;
  machineRepairEnabled: boolean;
  selectedNic: NetworkInterface | null;
  persistWanOnStartup: boolean;
  persistWanLoading: boolean;
  onDestChange: (value: string) => void;
  onMaskChange: (value: string) => void;
  onGatewayChange: (value: string) => void;
  onMetricChange: (value: string) => void;
  onBlurField: (field: keyof RouteValidationInput) => void;
  onAddRoute: () => void;
  onDeleteRoute: () => void;
  onSetWan: () => void;
  onFlushRoutes: () => void;
  onPersistWanChange: (checked: boolean) => void;
}

export function RouteConfigPanel({
  routeForm,
  visibleRouteFormErrors,
  machineRepairEnabled,
  persistWanOnStartup,
  persistWanLoading,
  onDestChange,
  onMaskChange,
  onGatewayChange,
  onMetricChange,
  onBlurField,
  onAddRoute,
  onDeleteRoute,
  onSetWan,
  onFlushRoutes,
  onPersistWanChange,
}: RouteConfigPanelProps) {
  return (
    <div className="p-3 border-b border-slate-700/30">
      <div className="route-form-grid mb-2">
        <Field
          fieldId="route-destination"
          label="Destination"
          value={routeForm.dest}
          onChange={onDestChange}
          onBlur={() => onBlurField("dest")}
          error={visibleRouteFormErrors.dest}
          placeholder="10.0.0.0"
        />
        <Field
          fieldId="route-mask"
          label="Subnet Mask"
          value={routeForm.mask}
          onChange={onMaskChange}
          onBlur={() => onBlurField("mask")}
          error={visibleRouteFormErrors.mask}
          placeholder="255.255.255.0"
        />
        <Field
          fieldId="route-gateway"
          label="Gateway"
          value={routeForm.gw}
          onChange={onGatewayChange}
          onBlur={() => onBlurField("gw")}
          error={visibleRouteFormErrors.gw}
          placeholder="192.168.1.1"
        />
        <Field
          fieldId="route-metric"
          label="Metric"
          value={routeForm.metric}
          onChange={onMetricChange}
          onBlur={() => onBlurField("metric")}
          error={visibleRouteFormErrors.metric}
          placeholder="10"
          inputMode="numeric"
        />
      </div>
      <div className="flex flex-wrap gap-1.5">
        <ActionBtn
          icon={Plus}
          label="ADD"
          color="emerald"
          onClick={onAddRoute}
          disabled={!machineRepairEnabled}
          compact
        />
        <ActionBtn
          icon={Trash2}
          label="DEL"
          color="red"
          onClick={onDeleteRoute}
          disabled={!machineRepairEnabled}
          compact
        />
        <ActionBtn
          icon={Globe}
          label="WAN"
          color="blue"
          disabled={!machineRepairEnabled}
          compact
          onClick={onSetWan}
        />
        <ActionBtn
          icon={Flame}
          label="FLUSH"
          color="orange"
          disabled={!machineRepairEnabled}
          compact
          onClick={onFlushRoutes}
        />
      </div>
      <div className="wan-persist-row">
        <label className="wan-persist-option">
          <input
            type="checkbox"
            checked={persistWanOnStartup}
            onChange={(event) => onPersistWanChange(event.target.checked)}
            disabled={persistWanLoading}
            className="w-3.5 h-3.5 rounded accent-blue-500"
          />
          <span>Persist on startup</span>
        </label>
        <span className="wan-persist-hint">
          Save or clear one unified startup replay config when you click WAN
        </span>
      </div>
    </div>
  );
}
