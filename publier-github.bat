@echo off
chcp 65001 >nul
cd /d "%~dp0"
where git >nul 2>nul || (echo Git n'est pas installe : https://git-scm.com/download/win & pause & exit /b 1)

if not exist ".git" (
    git init
    git branch -M main
)

git add -A
git commit -m "Roulette Sport v0.26" || echo (rien de nouveau a valider)

git remote get-url origin >nul 2>nul
if errorlevel 1 (
    echo.
    echo Cree d'abord un depot VIDE sur https://github.com/new
    echo ^(sans README, sans .gitignore, sans licence^)
    set /p URL=Colle l'adresse du depot ^(ex. https://github.com/pseudo/roulette-sport.git^) : 
    call git remote add origin %%URL%%
)

git push -u origin main
echo.
pause
