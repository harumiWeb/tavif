import { Button } from "antd";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { useAtom } from "jotai";
import { processedFilePathsSortedAtom, checkboxSelectedAtom , processedFilePathsAtom, isSavingAtom} from "@/app/atom";
import DownloadOutlined from "../icons/Download";
import { DeleteOutlined } from "@ant-design/icons";
import { Checkbox, CheckboxChangeEvent } from "antd";
import Null from "./Null";
import "@ant-design/v5-patch-for-react-19";
import { Modal } from "antd";

export default function OutputNavMenu() {
  const [processedFilePathsSorted, setProcessedFilePathsSorted] = useAtom(processedFilePathsSortedAtom);
  const [checkboxSelected, setCheckboxSelected] = useAtom(checkboxSelectedAtom);
  const [processedFilePaths, setProcessedFilePaths] = useAtom(processedFilePathsAtom);
  const [isSaving, setIsSaving] = useAtom(isSavingAtom);
  const [modal, modalContextHolder] = Modal.useModal();

  async function saveAll() {
    setIsSaving(true);
    const outputDir = await open({
      title: "Select Output Directory",
      directory: true,
      multiple: false,
    });
    if (!outputDir) {
      setIsSaving(false);
      return;
    }

    await invoke("save_files", {
      filePaths: processedFilePathsSorted,
      outputDir: outputDir,
    });
    setIsSaving(false);
    modal.success({
      title: "Success",
      centered: true,
      content: "Saved successfully",
    });
  }

  async function saveSelected() {
    setIsSaving(true);
    const outputDir = await open({
      title: "Select Output Directory",
      directory: true,
      multiple: false,
    });
    if (!outputDir) {
      setIsSaving(false);
      return;
    }

    const selectedFilePaths = checkboxSelected
      .filter((item) => item.checked)
      .map((item) => processedFilePathsSorted[item.index]);

    await invoke("save_files", {
      filePaths: selectedFilePaths,
      outputDir: outputDir,
    });
    setIsSaving(false);
    modal.success({
      title: "Success",
      centered: true,
      content: "Saved successfully",
    });
  }

  async function removeResult() {
    setProcessedFilePathsSorted([]);
    setCheckboxSelected([]);
    setProcessedFilePaths([]);
  }

  function handleCheckboxChange(e: CheckboxChangeEvent) {
    setCheckboxSelected((prev) =>
      prev.map((item) => ({
        ...item,
        checked: e.target.checked,
      }))
    );
  }

  if (processedFilePathsSorted.length === 0) return <Null />;

  return (
    <div className="absolute top-1 left-0 w-full h-fit p-2 flex items-center justify-between gap-2 bg-gray-50/50 backdrop-blur-sm border-l-2 border-r-2 border-gray-300 z-10">
      {modalContextHolder}
      <div className="flex items-center gap-2">
        <Button type="primary" onClick={saveAll} title="Save all files.">
          <DownloadOutlined size={16} className="fill-white" />
          Save ALL
        </Button>
        <Button type="primary" onClick={saveSelected} title="Save selected files.">
          <DownloadOutlined size={16} className="fill-white" />
          Save Selected
        </Button>
        <Button type="default" onClick={removeResult} title="Delete all processed files from the temp directory.">
          <DeleteOutlined size={16} className="fill-white" />
          Remove Result
        </Button>
      </div>
      <label className="flex items-center gap-2 cursor-pointer mr-3">
        All
        <Checkbox onChange={handleCheckboxChange} defaultChecked={true} />
      </label>
    </div>
  );
}
