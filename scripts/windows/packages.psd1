@{
    Order = @('Git', 'GitLfs', 'VisualStudio', 'CMake', 'Ninja', 'Python', 'Rustup')
    Packages = @{
        Git = @{
            Id = 'Git.Git'
            Command = 'git.exe'
            VersionArguments = @('--version')
            VersionPattern = '^git version '
            DependsOn = @()
            WingetArguments = @()
        }
        GitLfs = @{
            Id = 'GitHub.GitLFS'
            Command = 'git-lfs.exe'
            VersionArguments = @('--version')
            VersionPattern = '^git-lfs/'
            DependsOn = @('Git')
            WingetArguments = @()
        }
        VisualStudio = @{
            Id = 'Microsoft.VisualStudio.2022.BuildTools'
            Command = $null
            DependsOn = @()
            WingetArguments = @(
                '--override',
                '--wait --passive --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended'
            )
        }
        CMake = @{
            Id = 'Kitware.CMake'
            Command = 'cmake.exe'
            VersionArguments = @('--version')
            VersionPattern = '^cmake version '
            DependsOn = @()
            WingetArguments = @()
        }
        Ninja = @{
            Id = 'Ninja-build.Ninja'
            Command = 'ninja.exe'
            VersionArguments = @('--version')
            VersionPattern = '^\d+(?:\.\d+)+'
            DependsOn = @()
            WingetArguments = @()
        }
        Python = @{
            Id = 'Python.Python.3.13'
            Command = 'python.exe'
            VersionArguments = @('--version')
            VersionPattern = '^Python 3\.13(?:\.|$)'
            DependsOn = @()
            WingetArguments = @()
        }
        Rustup = @{
            Id = 'Rustlang.Rustup'
            Command = 'rustup.exe'
            VersionArguments = @('--version')
            VersionPattern = '^rustup '
            DependsOn = @('VisualStudio')
            WingetArguments = @()
        }
    }
}
