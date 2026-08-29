#!/usr/bin/env python3
"""
绿电直连新能源优化配置软件 - PDF 文档解析辅助工具
====================================================
功能：
  1. 从 PDF 提取文本（pymupdf）
  2. 从 PDF 提取内嵌图片（界面截图）
  3. 对图片执行中文 OCR（rapidocr，识别界面文字）

依赖（已安装到项目 .pyenv 目录，通过 PYTHONPATH 引用）：
  - pymupdf
  - rapidocr-onnxruntime

用法：
  PYTHONPATH=.pyenv python3 tools/parse_pdf.py <pdf路径> [--text <输出txt>] [--imgdir <图片输出目录>] [--ocr <OCR结果txt>]

示例：
  PYTHONPATH=.pyenv python3 tools/parse_pdf.py "docs/绿电直连新能源优化配置软件V2.2使用说明书.pdf" \
      --text /tmp/pdf_text.txt --imgdir /tmp/pdfimgs --ocr /tmp/ocr_results.txt
"""
import os, sys, glob, argparse, warnings
warnings.filterwarnings('ignore')

def extract_text(pdf, out_txt):
    import fitz
    doc = fitz.open(pdf)
    with open(out_txt, 'w', encoding='utf-8') as f:
        for i, page in enumerate(doc):
            f.write(f'\n===== PAGE {i+1} =====\n')
            f.write(page.get_text())
    print(f'text -> {out_txt} ({doc.page_count} pages)')

def extract_images(pdf, imgdir):
    import fitz
    doc = fitz.open(pdf)
    os.makedirs(imgdir, exist_ok=True)
    for pno in range(doc.page_count):
        page = doc[pno]
        for idx, img in enumerate(page.get_images(full=True)):
            xref = img[0]
            pix = fitz.Pixmap(doc, xref)
            if pix.n - pix.alpha > 3:
                pix = fitz.Pixmap(fitz.csRGB, pix)
            fn = os.path.join(imgdir, f'p{pno+1}_i{idx}.png')
            pix.save(fn)
    print(f'images -> {imgdir}')

def ocr_images(imgdir, out_ocr):
    from rapidocr_onnxruntime import RapidOCR
    ocr = RapidOCR()
    images = sorted(glob.glob(os.path.join(imgdir, '*.png')))
    with open(out_ocr, 'w', encoding='utf-8') as f:
        for img in images:
            name = os.path.basename(img)
            f.write(f'\n{"="*70}\n### {name}\n{"="*70}\n')
            try:
                result, _ = ocr(img)
                if result:
                    for line in result:
                        f.write(line[1] + '\n')
                else:
                    f.write('(无文字/纯图片)\n')
            except Exception as e:
                f.write(f'(OCR ERROR: {e})\n')
    print(f'ocr -> {out_ocr}')

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('pdf')
    ap.add_argument('--text')
    ap.add_argument('--imgdir')
    ap.add_argument('--ocr')
    args = ap.parse_args()
    if args.text:
        extract_text(args.pdf, args.text)
    if args.imgdir:
        extract_images(args.pdf, args.imgdir)
    if args.ocr:
        ocr_images(args.imgdir or 'imgdir_missing', args.ocr)

if __name__ == '__main__':
    main()
