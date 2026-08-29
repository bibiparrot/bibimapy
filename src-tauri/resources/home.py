import marimo

__generated_with = "bibimapy"
app = marimo.App(width="medium")


@app.cell
def _():
    import marimo as mo

    return (mo,)


@app.cell
def _(mo):
    import os

    locale = os.getenv("BIBIMAPY_LOCALE", "en")
    copy = {
        "en": ("Welcome to bibimapy", "Python 3.12 and marimo are managed locally by uv."),
        "zh-CN": ("欢迎使用 bibimapy", "Python 3.12 和 marimo 由 uv 在本地管理。"),
        "ja": ("bibimapy へようこそ", "Python 3.12 と marimo は uv によってローカル管理されます。"),
        "ko": ("bibimapy에 오신 것을 환영합니다", "Python 3.12와 marimo는 uv가 로컬에서 관리합니다."),
        "ru": ("Добро пожаловать в bibimapy", "Python 3.12 и marimo локально управляются uv."),
        "fr": ("Bienvenue dans bibimapy", "Python 3.12 et marimo sont gérés localement par uv."),
        "es": ("Bienvenido a bibimapy", "Python 3.12 y marimo se gestionan localmente con uv."),
        "pt": ("Bem-vindo ao bibimapy", "Python 3.12 e marimo são geridos localmente pelo uv."),
        "it": ("Benvenuto in bibimapy", "Python 3.12 e marimo sono gestiti localmente da uv."),
        "de": ("Willkommen bei bibimapy", "Python 3.12 und marimo werden lokal von uv verwaltet."),
        "la": ("Salve in bibimapy", "Python 3.12 et marimo a uv localiter administrantur."),
    }
    title, subtitle = copy.get(locale, copy["en"])
    mo.md(f"""
    # {title}

    {subtitle}

    Create reactive Python cells, explore data, and save notebooks in
    `~/.bibimapy/notebooks`.
    """)
    return


if __name__ == "__main__":
    app.run()
