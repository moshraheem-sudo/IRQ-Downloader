# IRQ Downloader

نسخة سطح مكتب مستقلة مبنية بلغة Rust من مشروع Ezoo Download للمطور Ahmed Ezzat. يحتفظ المشروع بإشعار المصدر الأصلي ورخصته في `LICENSE` و`THIRD-PARTY-NOTICES.txt`.

## البناء والتشغيل

- شغّل `BUILD.bat` لبناء إصدار Windows 64-bit.
- الناتج القابل للتشغيل: `dist\IRQ Downloader\IRQDownloader.exe`.
- يحتوي مجلد التوزيع أيضًا على `tools` (yt-dlp وFFmpeg وaria2) و`extension` لإضافة Chrome/Edge.
- لتشغيل الإصدار بعد بنائه، افتح `RUN_IRQ_DOWNLOAD.cmd` أو الملف التنفيذي مباشرة.

## ملفات التطوير التي تعتمد عليها

- `Cargo.toml` و`Cargo.lock`: إعداد مساحة عمل Rust وتثبيت إصدارات الاعتمادات.
- `crates\engine\src`: محرك التنزيلات والشبكة والاستئناف والفيديو والتورنت والتخزين.
- `crates\app\src`: تشغيل البرنامج وواجهة الربط مع Windows وإضافة المتصفح.
- `crates\app\ui`: شاشات Slint ونوافذ التطبيق.
- `crates\app\lang`: ترجمات الواجهة.
- `extension`: مصدر إضافة المتصفح المطابق لبروتوكول التطبيق. لا تعدّل الملفات المولّدة داخل `dist`؛ أعد البناء بعد تعديل المصدر.
- `assets`: ملفات الهوية الأصلية `IRQ Downloader App Icon.png` و`IRQ Downloader Logo.png`؛ الأيقونة المربعة الشفافة تُستخدم في التطبيق وموارده.
- `tools`: الأدوات التنفيذية المصاحبة التي تنسخ إلى حزمة التشغيل.
- `installer\irq_download.iss`: إعداد Inno Setup للمثبت عند الحاجة.
- `build_irq_release.ps1` و`BUILD.bat`: خطوات بناء الحزمة.

مصدر هذا التطبيق هو مساحة عمل Cargo أعلاه. الأصل الذي بُني عليه هو [Ezoo Download](https://github.com/AhmedEzo686/ezoo-download)، وتبقى الرخصة وإشعارات الأطراف الثالثة ضمن هذا المستودع.

## البيانات والاتصال

- الإعدادات وقائمة التنزيلات تحفظ في `%APPDATA%\IRQ\IRQDownloader\data`.
- في التشغيل الأول تُنسخ `settings.json` و`downloads.json` من مجلد Ezoo السابق إذا وُجدا.
- منفذ إضافة المتصفح المحلي الافتراضي `9624`، وتستخدم الإضافة اقترانًا آمنًا مع التطبيق.
- تحديث التطبيق التلقائي متوقف لهذه النسخة المستقلة كي لا تستبدلها إصدارات Ezoo. حدّثها ببناء/تثبيت إصدار IRQ Downloader.
