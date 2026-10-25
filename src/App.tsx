import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type Phase = "starting" | "ready" | "timeout" | "error";

interface StatusPayload {
  state: Phase;
  mode: string;
  message: string;
  log_path: string;
}

// 启动模式中文说明
const MODE_LABEL: Record<string, string> = {
  attach: "附着到已运行的 DeepTutor 服务",
  attach_pending: "检测到服务启动中，等待就绪",
  spawn: "正在自动拉起 DeepTutor 服务",
};

// 加载页状态机：starting → ready / timeout / error
function App() {
  const [status, setStatus] = useState<StatusPayload>({
    state: "starting",
    mode: "spawn",
    message: "",
    log_path: "",
  });

  useEffect(() => {
    const unlisten = listen<StatusPayload>("service:status", (e) =>
      setStatus(e.payload),
    );
    // 先取快照，防事件早发丢失
    invoke<StatusPayload>("get_status")
      .then(setStatus)
      .catch(() => {});
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  const retry = () => {
    setStatus({ state: "starting", mode: "spawn", message: "", log_path: "" });
    invoke("retry_start").catch(() => {});
  };

  const openLog = () => {
    invoke("open_service_log").catch(() => {});
  };

  return (
    <div className="flex h-full items-center justify-center bg-[#f6f7fb]">
      <div className="w-[420px] rounded-2xl bg-white p-10 text-center shadow-lg">
        {status.state === "ready" ? (
          <>
            <h1 className="text-xl font-semibold text-slate-800">
              即将进入 DeepTutor…
            </h1>
            <p className="mt-3 text-sm text-slate-500">
              服务已就绪，正在打开界面
            </p>
          </>
        ) : status.state === "timeout" || status.state === "error" ? (
          <>
            <h1 className="text-xl font-semibold text-rose-600">
              {status.state === "timeout" ? "启动超时" : "启动出错"}
            </h1>
            <p className="mt-3 text-sm text-slate-600">{status.message}</p>
            {status.log_path && (
              <p className="mt-2 break-all text-xs text-slate-400">
                服务日志：{status.log_path}
              </p>
            )}
            <p className="mt-2 text-xs text-slate-400">
              DeepTutor 自身日志在其工作区目录下
            </p>
            <div className="mt-6 flex justify-center gap-3">
              <button
                onClick={retry}
                className="rounded-lg bg-blue-600 px-5 py-2 text-sm text-white hover:bg-blue-700"
              >
                重试
              </button>
              <button
                onClick={openLog}
                className="rounded-lg border border-slate-300 px-5 py-2 text-sm text-slate-600 hover:bg-slate-50"
              >
                查看日志
              </button>
            </div>
          </>
        ) : (
          <>
            <h1 className="text-xl font-semibold text-slate-800">
              正在启动 DeepTutor 服务…
            </h1>
            <div className="mt-4 flex justify-center gap-2">
              <span className="h-2.5 w-2.5 animate-bounce rounded-full bg-blue-500 [animation-delay:-0.3s]" />
              <span className="h-2.5 w-2.5 animate-bounce rounded-full bg-blue-500 [animation-delay:-0.15s]" />
              <span className="h-2.5 w-2.5 animate-bounce rounded-full bg-blue-500" />
            </div>
            <p className="mt-4 text-sm text-slate-500">
              {MODE_LABEL[status.mode] ?? ""}
            </p>
          </>
        )}
      </div>
    </div>
  );
}

export default App;
