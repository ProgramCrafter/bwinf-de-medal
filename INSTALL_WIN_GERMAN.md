# Medal unter Windows (64 bit) zum laufen bringen

Voraussetzung: Rust und Cargo sind bereits installiert

## 1. vcpkg installieren:
Ausführliche Anleitung von Microsoft: https://docs.microsoft.com/en-us/cpp/build/vcpkg?view=vs-2019

Kurz:
1. Git-Repository von https://github.com/Microsoft/vcpkg in den eigenen Dokumente-Ordner klonen.
2. Im neuen Ordner „vcpkg“ die Datei „bootstrap-vcpkg.bat“ ausführen (wahlweise über die Windows-Eingabeaufforderung)

## 2. OpenSSL und sqlite3 mit vcpkg installieren:
1. Über die Windows-Eingabeaufforderung in den Ordner vcpkg navigieren
2. Folgende Kommandos eingeben:
```
vcpkg install openssl:x64-windows
vcpkg install sqlite:x64-windows
```

## 3. Umgebungsvariablen für die Eingabeaufforderung anpassen:
1. Im Datei-Explorer Rechtsklick auf „Dieser PC“ → Eigenschaften
2. Auf der linken Seite auf „Erweiterte Systemeinstellungen“ klicken
3. Im neuen Fenster „Systemeigenschaften“ den Reiter „Erweitert“ auswählen
4. Auf den untersten Button „Umgebungsvariablen“ klicken
5. In der unteren Tabelle „Systemvariablen“ folgende Variablen überprüfen/ändern:
   - OPENSSL_DIR sollte den Dateipfad des Ordners „vcpkg\packages\openssl-windows_x64-windows“ als Wert haben.
     Bei mir zum Beispiel: `C:\Users\<Nutzername>\Documents\vcpkg\packages\openssl-windows_x64-windows`
   - OPENSSL_LIB_DIR sollte den Dateipfad des Ordners „vcpkg\packages\openssl-windows_x64-windows/lib“ als Wert haben
   - SQLITE3_LIB_DIR sollte den Dateipfad des Ordners „vcpkg\packages\sqlite3_x64-windows\lib“ als Wert haben
6. Mit „Ok“ abschließen
7. Nicht vergessen, die Eingabeaufforderung einmal zu schließen und neu zu starten, damit die Umgebungsvariablen übernommen werden

## 4. sqlite3.dll an die richtige Stelle kopieren:
Die Datei „sqlite3.dll“ aus dem Ordner `\vcpkg\packages\sqlite3_x64-windows\bin` in den Ordner `medal-prototype` kopieren.
