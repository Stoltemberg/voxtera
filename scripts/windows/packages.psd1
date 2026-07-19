@{
    Order = @('Git', 'GitLfs', 'VisualStudio', 'CMake', 'Ninja', 'Python', 'Rustup')
    Packages = @{
        Git = @{
            Id = 'Git.Git'
            Command = 'git.exe'
            DependsOn = @()
            WingetArguments = @()
        }
        GitLfs = @{
            Id = 'GitHub.GitLFS'
            Command = 'git-lfs.exe'
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
            DependsOn = @()
            WingetArguments = @()
        }
        Ninja = @{
            Id = 'Ninja-build.Ninja'
            Command = 'ninja.exe'
            DependsOn = @()
            WingetArguments = @()
        }
        Python = @{
            Id = 'Python.Python.3.13'
            Command = 'python.exe'
            DependsOn = @()
            WingetArguments = @()
        }
        Rustup = @{
            Id = 'Rustlang.Rustup'
            Command = 'rustup.exe'
            DependsOn = @('VisualStudio')
            WingetArguments = @()
        }
    }
}
