# YOLO 模型訓練指南 - 測謊機制檢測

本文檔說明如何在本地訓練和部署 YOLO 模型來檢測新的遊戲測謊機制。

## 目錄
- [環境設置](#環境設置)
- [數據集準備](#數據集準備)
- [YOLO 模型訓練](#yolo-模型訓練)
- [模型轉換](#模型轉換)
- [集成到項目](#集成到項目)

---

## 環境設置

### 1. 安裝 Python 和必要的包

確保你有 Python 3.9+ 和 pip：

```bash
# 建議使用 Python 3.10 或 3.11
python --version
pip --version
```

### 2. 建立虛擬環境

```bash
# 在項目根目錄中建立虛擬環境
python -m venv yolo_env

# 啟動虛擬環境
# 在 Windows 上：
yolo_env\Scripts\activate

# 在 macOS/Linux 上：
source yolo_env/bin/activate
```

### 3. 安裝依賴包

```bash
# 升級 pip
pip install --upgrade pip

# 安裝 YOLO 及相關依賴
pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu121
pip install ultralytics opencv-python onnx onnxruntime
pip install opencv-contrib-python
pip install pillow numpy matplotlib

# 如果使用 GPU（推薦），確保安裝了 CUDA 工具包
# https://developer.nvidia.com/cuda-downloads
```

### 4. 驗證安裝

```python
import torch
import ultralytics
from ultralytics import YOLO

print(f"PyTorch version: {torch.__version__}")
print(f"CUDA available: {torch.cuda.is_available()}")
print(f"YOLOv8 imported successfully")
```

---

## 數據集準備

### 1. 數據收集

收集包含測謊機制的遊戲截圖：

```
lie_detector_dataset/
├── images/
│   ├── train/
│   │   ├── screenshot_001.png
│   │   ├── screenshot_002.png
│   │   └── ...
│   ├── val/
│   │   └── ...
│   └── test/
│       └── ...
└── labels/
    ├── train/
    │   ├── screenshot_001.txt
    │   ├── screenshot_002.txt
    │   └── ...
    ├── val/
    │   └── ...
    └── test/
        └── ...
```

### 2. 標註數據

使用標註工具為數據集標註邊界框。推薦使用 Roboflow 或 LabelMe：

```bash
# 安裝 LabelMe
pip install labelme

# 啟動 LabelMe
labelme
```

**標註步驟：**
- 打開圖像文件夾
- 為每個測謊機制元素繪製邊界框
- 標記類別（例如：shape, number, face, etc.）
- 保存為 YOLO 格式（`.txt` 文件）

### 3. YOLO 標籤格式

每個 `.txt` 標籤文件應包含格式如下的行：
```
<class_id> <x_center> <y_center> <width> <height>
```

其中坐標是相對於圖像尺寸的歸一化值（0-1）。

**示例 `screenshot_001.txt`：**
```
0 0.5 0.5 0.2 0.3
1 0.3 0.7 0.15 0.2
```

### 4. 建立 dataset.yaml 配置文件

在數據集根目錄建立 `dataset.yaml`：

```yaml
# dataset.yaml
path: D:\repos\komari-2\lie_detector_dataset
train: images/train
val: images/val
test: images/test

nc: 3  # 類別數
names: ['shape', 'number', 'face']  # 類別名稱
```

### 5. 數據集分割

```bash
# 如果圖像和標籤在同一個文件夾
python scripts/split_dataset.py

# 或手動複製到相應文件夾
```

**分割建議比例：**
- 訓練集：70%
- 驗證集：15%
- 測試集：15%

---

## YOLO 模型訓練

### 1. 基本訓練腳本

建立 `train_yolo.py`：

```python
from ultralytics import YOLO
import torch

def train_lie_detector_model():
    """訓練 YOLO 模型檢測測謊機制"""
    
    # 使用預訓練模型
    # yolov8n: nano (最快，精度較低)
    # yolov8s: small
    # yolov8m: medium (推薦)
    # yolov8l: large
    # yolov8x: extra large (最慢，精度最高)
    model = YOLO('yolov8m.pt')
    
    # 訓練配置
    results = model.train(
        data='lie_detector_dataset/dataset.yaml',
        epochs=100,
        imgsz=640,
        batch=16,  # 根據 GPU 記憶體調整
        device=0,  # GPU 設備 ID，-1 使用 CPU
        patience=20,  # Early stopping 耐心值
        save=True,
        save_period=10,
        exist_ok=True,
        project='runs/detect',
        name='lie_detector_v1',
        
        # 增強設定
        augment=True,
        mosaic=1.0,
        mixup=0.1,
        
        # 優化器設定
        optimizer='SGD',  # SGD, Adam, AdamW
        lr0=0.001,
        lrf=0.01,
        momentum=0.937,
        weight_decay=0.0005,
        
        # 驗證和監控
        val=True,
        conf=0.25,
        iou=0.6,
    )
    
    return model, results

if __name__ == '__main__':
    print(f"GPU 可用: {torch.cuda.is_available()}")
    if torch.cuda.is_available():
        print(f"GPU 設備: {torch.cuda.get_device_name(0)}")
    
    model, results = train_lie_detector_model()
    print("訓練完成!")
```

### 2. 運行訓練

```bash
# 確保虛擬環境已激活
python train_yolo.py
```

訓練輸出會保存在 `runs/detect/lie_detector_v1/` 中。

### 3. 訓練監控

在訓練過程中，TensorBoard 可視化訓練曲線：

```bash
# 在另一個終端中
tensorboard --logdir runs/detect
```

打開瀏覽器訪問 `http://localhost:6006`

### 4. 進階訓練選項

**微調現有模型：**
```python
# 從保存的檢查點繼續訓練
model = YOLO('runs/detect/lie_detector_v1/weights/best.pt')
results = model.train(
    data='dataset.yaml',
    epochs=50,
    imgsz=640,
    batch=16,
    resume=True,
)
```

**多尺度訓練：**
```python
# 使用不同的輸入尺寸
results = model.train(
    data='dataset.yaml',
    epochs=100,
    imgsz=[640, 672, 704],  # 多尺度
)
```

---

## 模型轉換

### 1. 轉換為 ONNX 格式

ONNX 是與框架無關的模型格式，可在 Rust 中使用：

```python
from ultralytics import YOLO

def export_to_onnx():
    """將訓練好的 YOLO 模型轉換為 ONNX"""
    
    # 加載最佳模型
    model = YOLO('runs/detect/lie_detector_v1/weights/best.pt')
    
    # 導出為 ONNX
    success, info = model.export(
        format='onnx',
        opset=13,  # ONNX opset 版本
        simplify=True,
        dynamic=False,
    )
    
    print(f"導出成功: {success}")
    print(f"模型信息: {info}")
    
    return model

if __name__ == '__main__':
    export_to_onnx()
```

### 2. 驗證 ONNX 模型

```python
import onnx
import onnxruntime as rt

def verify_onnx_model(onnx_path):
    """驗證 ONNX 模型"""
    
    # 檢查模型結構
    onnx_model = onnx.load(onnx_path)
    onnx.checker.check_model(onnx_model)
    print("ONNX 模型結構驗證通過")
    
    # 檢查運行時
    sess = rt.InferenceSession(onnx_path)
    input_name = sess.get_inputs()[0].name
    output_names = [o.name for o in sess.get_outputs()]
    
    print(f"輸入節點: {input_name}")
    print(f"輸出節點: {output_names}")

if __name__ == '__main__':
    verify_onnx_model('runs/detect/lie_detector_v1/weights/best.onnx')
```

### 3. 模型位置

訓練完成後，模型文件會生成在：

```
runs/detect/lie_detector_v1/weights/
├── best.pt          # 最佳 PyTorch 模型
├── last.pt          # 最後一個 PyTorch 模型
└── best.onnx        # 轉換後的 ONNX 模型
```

---

## 集成到項目

### 1. 複製模型文件

```bash
# 將 ONNX 模型複製到項目資源目錄
copy runs\detect\lie_detector_v1\weights\best.onnx backend\resources\lie_detector.onnx
```

### 2. 更新 Rust 代碼

在 `backend/src/solvers/` 中建立或更新 lie_detector 檢測器：

```rust
// backend/src/solvers/lie_detector.rs

use opencv::core::{Mat, Rect};
use crate::detect::Detector;

pub struct LieDetectorSolver {
    // 實現檢測邏輯
}

impl LieDetectorSolver {
    pub fn solve(&self, detector: &dyn Detector, region: Rect) -> Option<Vec<Detection>> {
        // 使用 lie_detector.onnx 模型進行推理
        let detections = detector.detect_lie_detector_elements(region);
        Some(detections)
    }
}

pub struct Detection {
    pub bbox: Rect,
    pub class_id: u32,
    pub confidence: f32,
    pub class_name: String,
}
```

### 3. 更新 Detector trait

在 `backend/src/detect.rs` 中添加新的檢測方法：

```rust
pub trait Detector {
    // ... 現有方法 ...
    
    fn detect_lie_detector_elements(&self, region: Rect) -> Vec<Detection>;
}
```

### 4. 加載和使用模型

```rust
// 在相關的檢測實現中
use ort::{Session, Value};

let session = Session::builder()?
    .with_execution_providers([ExecutionProvider::cuda(Default::default())])
    .commit_from_file("backend/resources/lie_detector.onnx")?;

// 運行推理
let input_tensor = /* 準備輸入 */;
let outputs = session.run(ort::inputs![input_tensor]?)?;
```

### 5. 模型元數據

建立 `backend/resources/lie_detector_metadata.json`：

```json
{
  "model": "lie_detector_yolov8m",
  "version": "1.0",
  "training_date": "2024-01-15",
  "input_size": 640,
  "classes": {
    "0": "shape",
    "1": "number",
    "2": "face"
  },
  "min_confidence": 0.25,
  "iou_threshold": 0.6
}
```

---

## 常見問題和優化

### 1. 性能優化

**快速推理：**
```bash
# 導出為 TensorRT 格式（如果使用 NVIDIA GPU）
python -c "from ultralytics import YOLO; YOLO('best.pt').export(format='engine')"
```

**量化模型：**
```python
# 動態量化
model = YOLO('best.pt')
model.export(format='onnx', dynamic=True)
```

### 2. 提高準確率

- 增加訓練數據量
- 增加訓練 epoch
- 使用更大的模型（yolov8l 或 yolov8x）
- 調整學習率和優化器
- 使用更強的數據增強

### 3. 類型不匹配錯誤

如果 ONNX 運行時出現類型錯誤：

```python
# 在導出時指定動態形狀
model.export(
    format='onnx',
    dynamic=True,
    opset=14,
)
```

### 4. 內存不足

如果訓練時 GPU 記憶體不足：

```python
results = model.train(
    batch=8,  # 減小 batch 大小
    imgsz=480,  # 減小輸入尺寸
    device=0,
)
```

---

## 驗證和測試

### 1. 驗證模型準確率

```python
from ultralytics import YOLO

model = YOLO('runs/detect/lie_detector_v1/weights/best.pt')
metrics = model.val()
```

### 2. 在測試圖像上運行推理

```python
from ultralytics import YOLO
import cv2

model = YOLO('runs/detect/lie_detector_v1/weights/best.pt')

# 檢測圖像
results = model.predict(source='test_image.png', conf=0.25)

# 可視化結果
for r in results:
    im_array = r.plot()
    cv2.imshow('Detection', im_array)
    cv2.waitKey(0)
```

### 3. 批量測試

```python
from ultralytics import YOLO
import os

model = YOLO('best.pt')

test_dir = 'lie_detector_dataset/images/test'
results = model.predict(
    source=test_dir,
    conf=0.25,
    save=True,
    project='predictions',
    name='test_results'
)
```

---

## 資源和參考

- [YOLOv8 官方文檔](https://docs.ultralytics.com)
- [YOLO 檢測教程](https://github.com/ultralytics/ultralytics)
- [ONNX 文檔](https://onnx.ai)
- [PyTorch 文檔](https://pytorch.org/docs/stable/index.html)

---

## 腳本位置建議

```
lie_detector_training/
├── train_yolo.py          # 主訓練腳本
├── export_onnx.py         # ONNX 導出腳本
├── verify_model.py        # 模型驗證腳本
├── inference.py           # 推理測試腳本
├── split_dataset.py       # 數據集分割腳本
├── dataset.yaml           # 數據集配置
└── lie_detector_dataset/  # 數據集目錄
    ├── images/
    │   ├── train/
    │   ├── val/
    │   └── test/
    └── labels/
        ├── train/
        ├── val/
        └── test/
```

---

## 許可證

遵循本項目的原始許可證條款。

---

最後更新：2024-06-17
